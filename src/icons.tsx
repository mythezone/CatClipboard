import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & {
  filled?: boolean;
};

export const ClipboardIcon = ({ filled, className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 24 24"
    fill={filled ? "currentColor" : "none"}
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <path d="M9 3.25a.75.75 0 0 1 .75-.75h4.5a.75.75 0 0 1 .75.75V5h1.75A2.25 2.25 0 0 1 19 7.25v11.5A2.25 2.25 0 0 1 16.75 21h-9.5A2.25 2.25 0 0 1 5 18.75V7.25A2.25 2.25 0 0 1 7.25 5H9V3.25Z" />
    <path d="M9 3.602A2.25 2.25 0 0 1 11.25 2h1.5A2.25 2.25 0 0 1 15 3.602V5h-6V3.602Z" />
  </svg>
);

export const StarIcon = ({ filled, className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 24 24"
    fill={filled ? "currentColor" : "none"}
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <path d="M11.049 2.927a1 1 0 0 1 1.902 0l1.519 4.674a1 1 0 0 0 .95.69h4.915a1 1 0 0 1 .588 1.81l-3.974 2.886a1 1 0 0 0-.363 1.118l1.519 4.674a1 1 0 0 1-1.538 1.118l-3.974-2.886a1 1 0 0 0-1.176 0l-3.974 2.886a1 1 0 0 1-1.538-1.118l1.519-4.674a1 1 0 0 0-.363-1.118L3.077 10.1a1 1 0 0 1 .588-1.81h4.915a1 1 0 0 0 .95-.69l1.519-4.674Z" />
  </svg>
);

export const TagIcon = ({ className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <path d="M3 10.75V6.5A2.5 2.5 0 0 1 5.5 4h4.25a2.5 2.5 0 0 1 1.768.732l8.75 8.75a2.5 2.5 0 0 1 0 3.536l-3.732 3.732a2.5 2.5 0 0 1-3.536 0l-8.75-8.75A2.5 2.5 0 0 1 3 10.75Z" />
    <circle cx={7.5} cy={7.5} r={1.25} />
  </svg>
);

export const TrashIcon = ({ className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <path d="M6.75 6.75h10.5" />
    <path d="M9.75 3.75h4.5a1.5 1.5 0 0 1 1.5 1.5v1.5H8.25v-1.5a1.5 1.5 0 0 1 1.5-1.5Z" />
    <path d="M18 6.75v11.5A2.75 2.75 0 0 1 15.25 21h-6.5A2.75 2.75 0 0 1 6 18.25V6.75" />
    <path d="M10 10.5v6" />
    <path d="M14 10.5v6" />
  </svg>
);

export const ChevronIcon = ({ className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <path d="M4.5 6 8 9.5 11.5 6" />
  </svg>
);

export const GearIcon = ({ className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <path d="M12 9.75a2.25 2.25 0 1 0 0 4.5 2.25 2.25 0 0 0 0-4.5Z" />
    <path d="M9.743 3.38c.453-1.145 2.06-1.145 2.513 0a1.724 1.724 0 0 0 2.591.85c1.012-.695 2.332.624 1.637 1.637a1.724 1.724 0 0 0 .85 2.59c1.145.454 1.145 2.061 0 2.514a1.724 1.724 0 0 0-.85 2.59c.695 1.013-.625 2.332-1.637 1.637a1.724 1.724 0 0 0-2.59.85c-.454 1.145-2.061 1.145-2.514 0a1.724 1.724 0 0 0-2.59-.85c-1.013.695-2.332-.624-1.637-1.637a1.724 1.724 0 0 0-.85-2.59c-1.145-.454-1.145-2.061 0-2.514a1.724 1.724 0 0 0 .85-2.59c-.695-1.012.624-2.332 1.637-1.637a1.724 1.724 0 0 0 2.59-.85Z" />
  </svg>
);

export const CatIcon = ({ className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 32 32"
    fill="currentColor"
    stroke="none"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <path d="M9.4 4.8c-.6-.3-1.3.1-1.3.8v6.2c0 .4-.2.7-.5.9-1.6 1.1-2.6 3-2.6 5.1 0 4.7 4.4 8.6 9.8 8.6h2.4c5.4 0 9.8-3.9 9.8-8.6 0-2.1-1-4-2.6-5.1-.3-.2-.5-.5-.5-.9V5.6c0-.7-.7-1.1-1.3-.8l-3.8 2c-.3.2-.7.2-1 0l-1.6-.9c-.6-.3-1.3-.3-1.9 0l-1.6.9c-.3.2-.7.2-1 0l-3.8-2Zm4.6 12.2a1.6 1.6 0 1 1-3.2 0 1.6 1.6 0 0 1 3.2 0Zm7.2 0a1.6 1.6 0 1 1-3.2 0 1.6 1.6 0 0 1 3.2 0ZM12 23.2c-.1-.5.5-.9.9-.6.8.5 1.8.8 3.1.8 1.3 0 2.3-.3 3.1-.8.5-.3 1 .1.9.6-.3 1.7-2.1 3-4 3s-3.7-1.3-4-3Z" />
  </svg>
);

export const SunIcon = ({ className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <circle cx={12} cy={12} r={4.5} />
    <path d="M12 2.25v2.5M12 19.25v2.5M4.75 4.75l1.77 1.77M17.48 17.48l1.77 1.77M2.25 12h2.5M19.25 12h2.5M4.75 19.25l1.77-1.77M17.48 6.52l1.77-1.77" />
  </svg>
);

export const MoonIcon = ({ className, ...rest }: IconProps) => (
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    aria-hidden="true"
    {...rest}
  >
    <path d="M20.593 15.667a8.25 8.25 0 0 1-11.26-11.26 8.25 8.25 0 1 0 11.26 11.26Z" />
  </svg>
);
