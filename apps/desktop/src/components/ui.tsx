import type {
  ButtonHTMLAttributes,
  HTMLAttributes,
  PropsWithChildren,
} from "react";

function join(...values: Array<string | false | undefined>) {
  return values.filter(Boolean).join(" ");
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "quiet" | "danger";
  size?: "default" | "small";
};

export function Button({
  className,
  variant = "quiet",
  size = "default",
  ...props
}: ButtonProps) {
  return (
    <button
      className={join("button", `button--${variant}`, `button--${size}`, className)}
      {...props}
    />
  );
}

export function Surface({
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return <div className={join("surface", className)} {...props} />;
}

export function Eyebrow({ children }: PropsWithChildren) {
  return <p className="eyebrow">{children}</p>;
}

export function StatusDot({
  tone,
}: {
  tone: "ready" | "checking" | "error";
}) {
  return <span className={join("status-dot", `status-dot--${tone}`)} aria-hidden />;
}

export function ArrowIcon() {
  return (
    <svg viewBox="0 0 16 16" aria-hidden>
      <path d="M3 8h9M9 4.5 12.5 8 9 11.5" />
    </svg>
  );
}

export function SendIcon() {
  return (
    <svg viewBox="0 0 18 18" aria-hidden>
      <path d="m3 9 11-5-3.2 10-2.1-3.2L3 9Z" />
      <path d="m8.7 10.8 2.4-2.5" />
    </svg>
  );
}
