import type { AgentStatus } from "../domain";
import { Button, StatusDot } from "./ui";

export function ReadinessBadge({
  status,
  onRetry,
  onAuthenticate,
}: {
  status: AgentStatus;
  onRetry: () => void;
  onAuthenticate: (methodId: string) => void;
}) {
  if (status.availability.status === "missing") {
    return (
      <div className="readiness readiness--error" role="status">
        <StatusDot tone="error" />
        <span>{status.availability.message}</span>
        <Button size="small" onClick={onRetry}>
          再確認
        </Button>
      </div>
    );
  }

  if (status.availability.status === "incompatible") {
    const label = status.runtime?.label ?? "Agent";
    return (
      <div className="readiness readiness--error" role="status">
        <StatusDot tone="error" />
        <span>
          {label} ACP {status.availability.found}（
          {status.availability.required}が必要）
        </span>
        <Button size="small" onClick={onRetry}>
          再確認
        </Button>
      </div>
    );
  }

  if (status.authentication.status === "required") {
    const preferred = status.authentication.methods[0];
    const label = status.runtime?.label ?? "Agent";
    return (
      <div className="readiness readiness--error" role="status">
        <StatusDot tone="error" />
        <span>{label}へログインしてください</span>
        {preferred && (
          <Button size="small" onClick={() => onAuthenticate(preferred.id)}>
            {preferred.name}
          </Button>
        )}
      </div>
    );
  }

  if (status.process.status === "failed") {
    return (
      <div className="readiness readiness--error" role="status">
        <StatusDot tone="error" />
        <span>{status.process.message}</span>
        <Button size="small" onClick={onRetry}>
          接続を再試行
        </Button>
      </div>
    );
  }

  const ready =
    status.authentication.status === "authenticated" &&
    ["ready", "busy"].includes(status.process.status);
  const provenance = status.runtime
    ? {
        nix_store: "Nix",
        packaged: "OpenBrief同梱",
        override: "カスタム",
      }[status.runtime.source]
    : null;
  const label = status.runtime?.label ?? "Agent";
  const version = status.runtime?.version
    ? ` ${status.runtime.version}`
    : "";
  return (
    <div className="readiness" role="status">
      <StatusDot tone={ready ? "ready" : "checking"} />
      <span>
        {ready
          ? `${label} ACP${version} · ${provenance ?? ""}`
          : status.authentication.status === "authenticating"
            ? `${label}へログイン中`
            : `${label}を確認中`}
      </span>
    </div>
  );
}
