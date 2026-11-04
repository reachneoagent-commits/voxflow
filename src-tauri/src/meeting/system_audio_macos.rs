//! Records everything the Mac is playing (the other side of a call) with a
//! Core Audio process tap. Needs macOS 14.4 or later and the "System Audio
//! Recording" permission; nothing is muted and no window is captured.

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_core_audio::{
    kAudioTapPropertyFormat, kAudioTapPropertyUID, AudioDeviceCreateIOProcID,
    AudioDeviceDestroyIOProcID, AudioDeviceIOProcID, AudioDeviceStart, AudioDeviceStop,
    AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap,
    AudioHardwareDestroyAggregateDevice, AudioHardwareDestroyProcessTap,
    AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress, CATapDescription,
    CATapMuteBehavior,
};
use objc2_core_audio_types::{AudioBufferList, AudioStreamBasicDescription, AudioTimeStamp};
use objc2_core_foundation::CFDictionary;
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};

const SCOPE_GLOBAL: u32 = 0x676c_6f62; // 'glob'
const ELEMENT_MAIN: u32 = 0;
const SYSTEM_OBJECT: AudioObjectID = 1;
const DEFAULT_SYSTEM_OUTPUT: u32 = 0x734f_7574; // 'sOut'
const DEVICE_UID: u32 = 0x7569_6420; // 'uid '
const FLAG_NON_INTERLEAVED: u32 = 1 << 5;

/// Audio handed over by the Core Audio thread: mono, at the tap's own rate.
struct Shared {
    samples: Mutex<Vec<f32>>,
    channels: u32,
    interleaved: bool,
}

/// A running recording of the Mac's sound output.
pub struct SystemAudioCapture {
    tap_id: AudioObjectID,
    aggregate_id: AudioObjectID,
    proc_id: AudioDeviceIOProcID,
    shared: Arc<Shared>,
    client_data: *const Shared,
    pub sample_rate: u32,
}

// The raw pointer is only ever the Arc we leaked on purpose and take back in `stop`.
unsafe impl Send for SystemAudioCapture {}

fn check(status: i32, what: &str) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{what} failed (error {status})"))
    }
}

unsafe fn property<T>(object: AudioObjectID, selector: u32) -> Result<T, String> {
    let mut value: T = std::mem::zeroed();
    let mut size = std::mem::size_of::<T>() as u32;
    let mut address = AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: SCOPE_GLOBAL,
        mElement: ELEMENT_MAIN,
    };
    check(
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&mut address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast(),
        ),
        "reading an audio property",
    )?;
    Ok(value)
}

/// Core Audio hands back an owned CFString; CFString and NSString are the same object.
unsafe fn string_property(object: AudioObjectID, selector: u32) -> Result<String, String> {
    let raw: *mut c_void = property(object, selector)?;
    let raw = NonNull::new(raw).ok_or("an audio property was empty")?;
    let string: Retained<NSString> =
        Retained::from_raw(raw.as_ptr().cast()).ok_or("an audio property was empty")?;
    Ok(string.to_string())
}

unsafe extern "C-unwind" fn on_audio(
    _device: AudioObjectID,
    _now: NonNull<AudioTimeStamp>,
    input: NonNull<AudioBufferList>,
    _input_time: NonNull<AudioTimeStamp>,
    _output: NonNull<AudioBufferList>,
    _output_time: NonNull<AudioTimeStamp>,
    client: *mut c_void,
) -> i32 {
    let shared = &*(client as *const Shared);
    let list = input.as_ref();
    let buffers = std::slice::from_raw_parts(list.mBuffers.as_ptr(), list.mNumberBuffers as usize);
    let Some(first) = buffers.first() else {
        return 0;
    };
    let mut mono: Vec<f32> = Vec::new();
    if shared.interleaved {
        let channels = shared.channels.max(1) as usize;
        let frames = first.mDataByteSize as usize / 4 / channels;
        let data = std::slice::from_raw_parts(first.mData as *const f32, frames * channels);
        mono.extend(
            data.chunks_exact(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32),
        );
    } else {
        let frames = first.mDataByteSize as usize / 4;
        mono.resize(frames, 0.0);
        for buffer in buffers {
            let data = std::slice::from_raw_parts(buffer.mData as *const f32, frames);
            for (out, sample) in mono.iter_mut().zip(data) {
                *out += sample / buffers.len() as f32;
            }
        }
    }
    if let Ok(mut samples) = shared.samples.lock() {
        samples.extend_from_slice(&mono);
    }
    0
}

impl SystemAudioCapture {
    pub fn start() -> Result<Self, String> {
        unsafe {
            let description = CATapDescription::initMonoGlobalTapButExcludeProcesses(
                CATapDescription::alloc(),
                &NSArray::<NSNumber>::new(),
            );
            description.setPrivate(true);
            description.setMuteBehavior(CATapMuteBehavior::Unmuted);
            description.setName(&NSString::from_str("Voxflow meeting"));

            let mut tap_id: AudioObjectID = 0;
            check(
                AudioHardwareCreateProcessTap(Some(&description), &mut tap_id),
                "creating the system audio tap",
            )?;

            let built = Self::build(tap_id, &description);
            if built.is_err() {
                AudioHardwareDestroyProcessTap(tap_id);
            }
            built
        }
    }

    unsafe fn build(tap_id: AudioObjectID, description: &CATapDescription) -> Result<Self, String> {
        let format: AudioStreamBasicDescription =
            property(tap_id, kAudioTapPropertyFormat).unwrap_or_else(|_| std::mem::zeroed());
        let sample_rate = if format.mSampleRate > 0.0 {
            format.mSampleRate as u32
        } else {
            48_000
        };
        let channels = format.mChannelsPerFrame.max(1);
        let interleaved = format.mFormatFlags & FLAG_NON_INTERLEAVED == 0;

        let tap_uid = description.UUID().UUIDString();
        let _ = kAudioTapPropertyUID;

        let mut entries: Vec<(&str, Retained<AnyObject>)> = Vec::new();
        let aggregate_uid =
            NSString::from_str(&format!("com.voxflow.meeting.{}", std::process::id()));
        entries.push(("uid", Retained::cast_unchecked(aggregate_uid)));
        entries.push((
            "name",
            Retained::cast_unchecked(NSString::from_str("Voxflow meeting")),
        ));
        entries.push(("private", Retained::cast_unchecked(NSNumber::new_i32(1))));
        entries.push((
            "tapautostart",
            Retained::cast_unchecked(NSNumber::new_i32(1)),
        ));

        // The default output device keeps the aggregate's clock running.
        if let Ok(output) = property::<AudioObjectID>(SYSTEM_OBJECT, DEFAULT_SYSTEM_OUTPUT) {
            if let Ok(uid) = string_property(output, DEVICE_UID) {
                let uid = NSString::from_str(&uid);
                entries.push(("master", Retained::cast_unchecked(uid.clone())));
                let sub = NSDictionary::from_retained_objects(
                    &[&*NSString::from_str("uid")],
                    &[Retained::cast_unchecked::<AnyObject>(uid)],
                );
                entries.push((
                    "subdevices",
                    Retained::cast_unchecked(NSArray::from_retained_slice(&[sub])),
                ));
            }
        }

        let tap_entry = NSDictionary::from_retained_objects(
            &[&*NSString::from_str("uid"), &*NSString::from_str("drift")],
            &[
                Retained::cast_unchecked::<AnyObject>(tap_uid),
                Retained::cast_unchecked::<AnyObject>(NSNumber::new_i32(1)),
            ],
        );
        entries.push((
            "taps",
            Retained::cast_unchecked(NSArray::from_retained_slice(&[tap_entry])),
        ));

        let keys: Vec<Retained<NSString>> =
            entries.iter().map(|(k, _)| NSString::from_str(k)).collect();
        let key_refs: Vec<&NSString> = keys.iter().map(|k| &**k).collect();
        let values: Vec<Retained<AnyObject>> = entries.into_iter().map(|(_, v)| v).collect();
        let dictionary = NSDictionary::from_retained_objects(&key_refs, &values);

        let mut aggregate_id: AudioObjectID = 0;
        check(
            AudioHardwareCreateAggregateDevice(
                &*(Retained::as_ptr(&dictionary) as *const CFDictionary),
                NonNull::from(&mut aggregate_id),
            ),
            "creating the recording device",
        )?;

        let shared = Arc::new(Shared {
            samples: Mutex::new(Vec::new()),
            channels,
            interleaved,
        });
        let client_data = Arc::into_raw(Arc::clone(&shared));
        let mut proc_id: AudioDeviceIOProcID = None;
        let started = check(
            AudioDeviceCreateIOProcID(
                aggregate_id,
                Some(on_audio),
                client_data as *mut c_void,
                NonNull::from(&mut proc_id),
            ),
            "starting the system audio recording",
        )
        .and_then(|()| {
            check(
                AudioDeviceStart(aggregate_id, proc_id),
                "starting the system audio recording",
            )
        });
        if let Err(e) = started {
            drop(Arc::from_raw(client_data));
            AudioHardwareDestroyAggregateDevice(aggregate_id);
            return Err(e);
        }

        Ok(Self {
            tap_id,
            aggregate_id,
            proc_id,
            shared,
            client_data,
            sample_rate,
        })
    }

    /// Stops recording and returns the mono audio at `sample_rate`.
    pub fn stop(self) -> Vec<f32> {
        unsafe {
            AudioDeviceStop(self.aggregate_id, self.proc_id);
            AudioDeviceDestroyIOProcID(self.aggregate_id, self.proc_id);
            AudioHardwareDestroyAggregateDevice(self.aggregate_id);
            AudioHardwareDestroyProcessTap(self.tap_id);
            drop(Arc::from_raw(self.client_data));
        }
        let mut samples = self
            .shared
            .samples
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        std::mem::take(&mut *samples)
    }
}
