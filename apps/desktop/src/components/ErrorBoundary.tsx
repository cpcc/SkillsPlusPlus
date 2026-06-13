import { Component, type ReactNode } from "react";

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
          <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
            <p className="text-lg font-semibold text-red-700">出错了</p>
            <p className="text-sm text-gray-500">{this.state.error?.message}</p>
          </div>
        )
      );
    }
    return this.props.children;
  }
}
