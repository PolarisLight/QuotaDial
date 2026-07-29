type BrandMarkProps = {
  className?: string;
};

export function BrandMark({ className }: BrandMarkProps) {
  return (
    <svg
      aria-label="Codex Monitor 余量窗口"
      className={className}
      role="img"
      viewBox="0 0 44 44"
      xmlns="http://www.w3.org/2000/svg"
    >
      <rect className="brand-mark__frame" x="5" y="5" width="34" height="34" rx="9" />
      <rect className="brand-mark__well" x="9" y="9" width="26" height="26" rx="6" />
      <path
        className="brand-mark__level"
        d="M9 25.3c4.1 0 5.9-4.2 9.9-4.2 4.1 0 5.8 4.1 9.9 4.1 2.5 0 4.4-.9 6.2-2.2v6a6 6 0 0 1-6 6H15a6 6 0 0 1-6-6v-3.7Z"
      />
      <circle className="brand-mark__glint" cx="30.5" cy="13.5" r="1.7" />
    </svg>
  );
}
