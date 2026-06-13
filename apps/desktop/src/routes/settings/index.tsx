import { useAppInfo } from "../../hooks/use-app-info";
import { Info, Database, Monitor } from "lucide-react";

export default function SettingsPage() {
  const { data, isLoading, error } = useAppInfo();

  return (
    <div className="mx-auto max-w-[680px]">
      <h1 className="text-xl font-semibold tracking-tight text-[var(--color-text-primary)]">
        设置
      </h1>
      <p className="mt-1 text-[13px] text-[var(--color-text-secondary)]">
        来源站配置、缓存管理与日志
      </p>

      {/* App Info */}
      <div className="mt-8">
        <h3 className="mb-3 text-[12px] font-medium uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
          应用信息
        </h3>
        <div className="rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] divide-y divide-[var(--color-border-subtle)]">
          {isLoading && (
            <div className="px-5 py-4">
              <div className="animate-pulse space-y-3">
                <div className="h-4 w-32 rounded bg-[var(--color-border-subtle)]" />
                <div className="h-4 w-24 rounded bg-[var(--color-border-subtle)]" />
                <div className="h-4 w-48 rounded bg-[var(--color-border-subtle)]" />
              </div>
            </div>
          )}
          {error && (
            <div className="flex items-center gap-3 px-5 py-4">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--color-danger-subtle)]">
                <Info className="h-4 w-4 text-[var(--color-danger)]" />
              </div>
              <p className="text-[13px] text-[var(--color-danger)]">
                加载失败：{String(error)}
              </p>
            </div>
          )}
          {data && (
            <>
              <InfoRow icon={Info} label="版本">
                <span className="text-[13px] text-[var(--color-text-secondary)]">
                  {data.version}
                </span>
              </InfoRow>
              <InfoRow icon={Monitor} label="平台">
                <span className="text-[13px] text-[var(--color-text-secondary)]">
                  {data.platform}
                </span>
              </InfoRow>
              <InfoRow icon={Database} label="数据库">
                <span className="truncate font-mono text-[11px] text-[var(--color-text-tertiary)]">
                  {data.dbPath}
                </span>
              </InfoRow>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function InfoRow({
  icon: Icon,
  label,
  children,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-4 px-5 py-3.5">
      <Icon className="h-4 w-4 shrink-0 text-[var(--color-text-tertiary)]" />
      <span className="w-20 shrink-0 text-[12px] font-medium text-[var(--color-text-tertiary)]">
        {label}
      </span>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
