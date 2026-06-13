import * as Toast from "@radix-ui/react-toast";
import { createContext, useContext, useState, useCallback } from "react";
import { CheckCircle, XCircle } from "lucide-react";

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
      <Toast.Provider swipeDirection="right">
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
            <div className="min-w-0">
              <Toast.Title className="text-[13px] font-semibold text-[var(--color-text-primary)]">
                {t.title}
              </Toast.Title>
              {t.description && (
                <Toast.Description className="mt-0.5 text-[12px] text-[var(--color-text-secondary)]">
                  {t.description}
                </Toast.Description>
              )}
            </div>
          </Toast.Root>
        ))}
        <Toast.Viewport className="fixed bottom-4 right-4 z-50 flex w-80 flex-col gap-2" />
      </Toast.Provider>
    </ToastContext.Provider>
  );
}
