import * as Toast from "@radix-ui/react-toast";
import { createContext, useContext, useState, useCallback } from "react";
import { CheckCircle, XCircle, X } from "lucide-react";

type ToastItem = {
  id: string;
  title: string;
  description?: string;
  variant?: "default" | "error";
};

type ToastFn = (
  title: string,
  description?: string,
  variant?: ToastItem["variant"],
) => void;

const ToastContext = createContext<ToastFn>(() => {});

export function useToast() {
  return useContext(ToastContext);
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const addToast = useCallback<ToastFn>(
    (title, description, variant = "default") => {
      const id = crypto.randomUUID();
      setToasts((prev) => [...prev, { id, title, description, variant }]);
    },
    [],
  );

  return (
    <ToastContext.Provider value={addToast}>
      <Toast.Provider swipeDirection="right" duration={5000}>
        {children}
        {toasts.map((t) => (
          <Toast.Root
            key={t.id}
            className={`flex items-start gap-3 rounded-[var(--radius-lg)] border p-4 shadow-xl shadow-black/20 ${
              t.variant === "error"
                ? "border-[var(--color-danger)]/20 bg-[var(--color-surface-overlay)]"
                : "border-[var(--color-border-default)] bg-[var(--color-surface-overlay)]"
            }`}
            onOpenChange={(open) => {
              if (!open)
                setToasts((prev) => prev.filter((x) => x.id !== t.id));
            }}
          >
            {t.variant === "error" ? (
              <XCircle className="mt-0.5 h-4 w-4 shrink-0 text-[var(--color-danger)]" />
            ) : (
              <CheckCircle className="mt-0.5 h-4 w-4 shrink-0 text-[var(--color-success)]" />
            )}
            <div className="min-w-0 flex-1">
              <Toast.Title className="text-[13px] font-semibold text-[var(--color-text-primary)]">
                {t.title}
              </Toast.Title>
              {t.description && (
                <Toast.Description className="mt-0.5 break-words text-[12px] text-[var(--color-text-secondary)]">
                  {t.description}
                </Toast.Description>
              )}
            </div>
            <Toast.Close className="shrink-0 rounded-[var(--radius-sm)] p-0.5 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-secondary)]">
              <X className="h-3.5 w-3.5" />
            </Toast.Close>
          </Toast.Root>
        ))}
        <Toast.Viewport className="fixed right-4 top-4 z-50 flex w-80 flex-col gap-2" />
      </Toast.Provider>
    </ToastContext.Provider>
  );
}
