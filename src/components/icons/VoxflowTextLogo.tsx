import VoxflowMark from "./VoxflowMark";

/** The mark with the "Voxflow" wordmark beside it. `width` sizes the mark. */
const VoxflowTextLogo = ({
  width = 34,
  className,
}: {
  width?: number;
  className?: string;
}) => (
  <span
    className={`inline-flex items-center gap-2 font-display ${className ?? ""}`}
  >
    <VoxflowMark
      width={width}
      className="text-logo-primary shrink-0 translate-y-[2px]"
    />
    {/* eslint-disable-next-line i18next/no-literal-string */}
    <span className="text-[17px] font-bold leading-none tracking-tight text-text">
      Voxflow
    </span>
  </span>
);

export default VoxflowTextLogo;
