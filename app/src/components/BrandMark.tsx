type BrandMarkProps = {
  className?: string;
};

const ticks = [
  "M22 5.5v4.8",
  "M36.5 14l-4.2 2.4",
  "M36.5 30l-4.2-2.4",
  "M22 38.5v-4.8",
  "M7.5 30l4.2-2.4",
  "M7.5 14l4.2 2.4",
];

export function BrandMark({ className }: BrandMarkProps) {
  return (
    <svg
      aria-label="Codex Monitor 额度表盘"
      className={className}
      role="img"
      viewBox="0 0 44 44"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        className="brand-mark__track"
        d="M22 5.5 36.5 14v16L22 38.5 7.5 30V14Z"
        pathLength="100"
      />
      <path
        className="brand-mark__used"
        d="M22 5.5 36.5 14v16L22 38.5 7.5 30V14Z"
        pathLength="100"
      />
      {ticks.map((path) => (
        <path className="brand-mark__tick" d={path} key={path} />
      ))}
      <path className="brand-mark__hand" d="M24.5 20 13.1 29.3" />
      <circle className="brand-mark__hub" cx="22" cy="22" r="2.25" />
      <circle className="brand-mark__hub-core" cx="22" cy="22" r=".95" />
    </svg>
  );
}
