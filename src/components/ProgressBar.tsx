import { motion } from 'framer-motion';
import { cn } from '@/lib/utils';

interface ProgressBarProps {
  percentage: number;
  className?: string;
}

export function ProgressBar({ percentage, className }: ProgressBarProps) {
  return (
    <div className={cn('h-1.5 bg-bg-tertiary rounded-full overflow-hidden', className)}>
      <motion.div
        className="h-full bg-accent"
        initial={{ width: 0 }}
        animate={{ width: `${percentage}%` }}
        transition={{ duration: 0.2 }}
      />
    </div>
  );
}
