import * as Dialog from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import type { ReactNode } from "react";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title?: ReactNode;
  children: ReactNode;
  /** Tailwind width class; default `w-[480px]`. */
  widthClass?: string;
}

/**
 * 右侧滑出抽屉。基于 Radix Dialog；`data-state=open/closed` 触发
 * CSS 过渡（见 App.css 中 `.drawer-panel` / `.drawer-overlay`）。
 */
export function Drawer({
  open,
  onOpenChange,
  title,
  children,
  widthClass = "w-[480px]",
}: Props) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="drawer-overlay fixed inset-0 z-40 bg-black/50 backdrop-blur-sm" />
        <Dialog.Content
          className={`drawer-panel fixed right-0 top-0 z-50 flex h-full ${widthClass} flex-col border-l border-[var(--color-border-subtle)] bg-[var(--color-surface-overlay)] shadow-2xl shadow-black/40 focus:outline-none`}
        >
          {title !== undefined && (
            <div className="flex items-start justify-between gap-3 border-b border-[var(--color-border-subtle)] px-5 py-4">
              <div className="min-w-0 flex-1">{title}</div>
              <Dialog.Close asChild>
                <button
                  className="rounded-[var(--radius-sm)] p-1 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-secondary)]"
                  aria-label="关闭"
                >
                  <X className="h-4 w-4" />
                </button>
              </Dialog.Close>
            </div>
          )}
          <div className="min-h-0 flex-1 overflow-auto">{children}</div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
