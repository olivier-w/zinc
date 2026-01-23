import { memo } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import type { Toast as ToastType } from '@/lib/types';
import { cn } from '@/lib/utils';
import { CheckIcon, XIcon, AlertCircleIcon, InfoIcon } from './Icons';

interface ToastProps {
  toast: ToastType;
  onDismiss: (id: string) => void;
}

const iconMap = {
  success: CheckIcon,
  error: XIcon,
  warning: AlertCircleIcon,
  info: InfoIcon,
};

const colorMap = {
  success: 'bg-success/10 border-success/20 text-success',
  error: 'bg-error/10 border-error/20 text-error',
  warning: 'bg-warning/10 border-warning/20 text-warning',
  info: 'bg-accent/10 border-accent/20 text-accent',
};

const ToastItem = memo(function ToastItem({ toast, onDismiss }: ToastProps) {
  const Icon = iconMap[toast.type];

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: -20, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, x: 100, scale: 0.95 }}
      transition={{ type: 'spring', stiffness: 500, damping: 30 }}
      className={cn(
        'flex items-center gap-3 px-4 py-3 rounded-lg border shadow-lg',
        'backdrop-blur-md min-w-[280px] max-w-[400px]',
        colorMap[toast.type]
      )}
    >
      <Icon className="w-5 h-5 shrink-0" />
      <p className="text-sm flex-1 text-text-primary">{toast.message}</p>
      <button
        onClick={() => onDismiss(toast.id)}
        className="p-1 rounded hover:bg-white/10 transition-colors"
        aria-label="Dismiss notification"
      >
        <XIcon className="w-4 h-4" />
      </button>
    </motion.div>
  );
});

interface ToastContainerProps {
  toasts: ToastType[];
  onDismiss: (id: string) => void;
}

export function ToastContainer({ toasts, onDismiss }: ToastContainerProps) {
  return (
    <div
      className="fixed top-4 right-4 z-50 flex flex-col gap-2"
      role="region"
      aria-label="Notifications"
    >
      <AnimatePresence mode="popLayout">
        {toasts.map(toast => (
          <ToastItem key={toast.id} toast={toast} onDismiss={onDismiss} />
        ))}
      </AnimatePresence>
    </div>
  );
}
