import * as Toast from "@radix-ui/react-toast";
import { createContext, useContext, useState, useCallback } from "react";

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
            className={`rounded-lg border p-4 shadow-lg ${
              t.variant === "error"
                ? "border-red-200 bg-red-50 text-red-900"
                : "border-gray-200 bg-white text-gray-900"
            }`}
            onOpenChange={(open) => {
              if (!open)
                setToasts((prev) => prev.filter((x) => x.id !== t.id));
            }}
          >
            <Toast.Title className="text-sm font-semibold">{t.title}</Toast.Title>
            {t.description && (
              <Toast.Description className="mt-1 text-xs text-gray-500">
                {t.description}
              </Toast.Description>
            )}
          </Toast.Root>
        ))}
        <Toast.Viewport className="fixed bottom-4 right-4 z-50 flex w-80 flex-col gap-2" />
      </Toast.Provider>
    </ToastContext.Provider>
  );
}
