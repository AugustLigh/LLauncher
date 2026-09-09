const paths = {
  play: "m8 5 11 7-11 7Z",
  pause: "M9 5v14M15 5v14",
  close: "m6 6 12 12M18 6 6 18",
  arrow: "M5 12h14m-6-6 6 6-6 6",
  back: "M19 12H5m6-6-6 6 6 6",
  external: "M14 4h6v6m0-6L10 14M10 4H4v16h16v-6",
  settings:
    "m9 3-1 3-3 1-2 5 2 5 3 1 1 3h6l1-3 3-1 2-5-2-5-3-1-1-3Zm3 5a4 4 0 1 0 0 8 4 4 0 0 0 0-8",
  check: "m5 12 4 4L19 6",
  alert: "M12 8v5m0 4h.01M12 3 2 21h20Z",
  folder: "M3 6h7l2 3h9v11H3ZM3 9V4h7l2 2h9v3",
  download: "M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5",
  refresh: "M20 7v5h-5M4 17v-5h5m10-5a8 8 0 0 0-14-2m0 14a8 8 0 0 0 14-2",
  chevron: "m6 9 6 6 6-6",
  clock: "M12 7v5l3 2m6-2a9 9 0 1 1-18 0 9 9 0 0 1 18 0",
  file: "M14 3H5v18h14V8Zm0 0v5h5M8 13h8M8 17h6",
  mods: "m12 3 9 5v9l-9 5-9-5V8Zm0 10 9-5M3 8l9 5m0 0v9",
  monitor: "M3 4h18v13H3Zm9 13v4m-5 0h10",
  globe:
    "M3 12h18m-9-9c-6 6-6 12 0 18 6-6 6-12 0-18Zm9 9a9 9 0 1 1-18 0 9 9 0 0 1 18 0",
  minus: "M5 12h14",
  stop: "M6 6h12v12H6Z",
  copy: "M8 8h13v13H8ZM16 8V3H3v13h5",
  menu: "M5 7h14M5 12h14M5 17h10",
};
export default function Icon({ name, size = 18, className = "" }) {
  return (
    <svg
      className={`icon ${className}`}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name] || paths.file} />
    </svg>
  );
}
