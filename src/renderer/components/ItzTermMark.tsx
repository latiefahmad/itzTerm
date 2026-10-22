/**
 * itzTerm brand mark.
 *
 * Inline SVG so it can inherit the current text color via `currentColor`,
 * which keeps it legible on both light and dark themes. Geometry matches the
 * iT monogram from `src-tauri/icons/icon.svg` (letter forms only, no tile).
 */
export function ItzTermMark({
  size = 22,
  className
}: {
  size?: number
  className?: string
}): React.JSX.Element {
  return (
    <svg
      width={size}
      height={size}
      viewBox="282 293 507 451"
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      aria-label="itzTerm"
      className={className}
    >
      <circle cx="355" cy="355" r="54" fill="currentColor" />
      <path
        d="M324 454 H362 Q396 454 396 488 V735 H352 C315 735 290 708 290 669 V488 Q290 454 324 454 Z"
        fill="currentColor"
      />
      <path
        d="M453 311 H749 Q781 311 781 340 V386 Q781 416 749 416 H654 V711 Q654 736 629 736 H577 Q552 736 552 711 V416 H453 Q425 416 425 390 V339 Q425 311 453 311 Z"
        fill="currentColor"
      />
    </svg>
  )
}
