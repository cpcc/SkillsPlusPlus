import { Component, type ReactNode } from "react";
import { AlertTriangle } from "lucide-react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback ?? (
          <div className="flex h-screen flex-col items-center justify-center gap-4 bg-[var(--color-surface-base)] p-8 text-center">
            <div className="flex h-14 w-14 items-center justify-center rounded-xl border border-[var(--color-danger)]/20 bg-[var(--color-danger-subtle)]">
              <AlertTriangle className="h-6 w-6 text-[var(--color-danger)]" />
            </div>
            <div>
              <p className="text-[15px] font-semibold text-[var(--color-text-primary)]">
                出错了
              </p>
              <p className="mt-1 max-w-sm text-[13px] text-[var(--color-text-secondary)]">
                {this.state.error?.message}
              </p>
            </div>
          </div>
        )
      );
    }
    return this.props.children;
  }
}
