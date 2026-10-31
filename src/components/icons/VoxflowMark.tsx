/** The Voxflow mark: five voice-wave ellipses. Takes its colour from the text colour. */
const VoxflowMark = ({
  width,
  height,
  className,
}: {
  width?: number | string;
  height?: number | string;
  className?: string;
}) => (
  <svg
    width={width}
    height={height}
    viewBox="0 0 104.29 52.93"
    fill="currentColor"
    className={className}
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <ellipse cx="8.63" cy="26.52" rx="8.63" ry="12.13" />
    <ellipse cx="29.93" cy="26.46" rx="9.76" ry="21.89" />
    <ellipse cx="55.08" cy="26.46" rx="12.49" ry="26.46" />
    <ellipse cx="79.1" cy="26.52" rx="8.63" ry="17.46" />
    <ellipse cx="97.46" cy="26.46" rx="6.83" ry="11.03" />
  </svg>
);

export default VoxflowMark;
