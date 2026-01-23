import { motion } from 'motion/react';
import { cn } from '@/lib/utils';

type ProgressRingStatus = 'downloading' | 'transcribing' | 'completed' | 'error' | 'pending' | 'cancelled';

interface ProgressRingProps {
  status: ProgressRingStatus;
  progress?: number; // 0-100
  size?: number;
  className?: string;
}

export function ProgressRing({
  status,
  progress = 0,
  size = 24,
  className,
}: ProgressRingProps) {
  const strokeWidth = 2.5;
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const center = size / 2;

  // Calculate stroke offset for progress
  const strokeDashoffset = circumference - (progress / 100) * circumference;

  if (status === 'completed') {
    return (
      <motion.div
        initial={{ scale: 0 }}
        animate={{ scale: 1 }}
        transition={{ type: 'spring', stiffness: 500, damping: 25 }}
        className={cn('completion-burst', className)}
      >
        <svg
          width={size}
          height={size}
          viewBox={`0 0 ${size} ${size}`}
          fill="none"
          className="text-success"
        >
          {/* Background circle */}
          <circle
            cx={center}
            cy={center}
            r={radius}
            fill="currentColor"
            opacity={0.2}
          />
          {/* Checkmark */}
          <motion.path
            d={`M${size * 0.28} ${size * 0.5} L${size * 0.44} ${size * 0.65} L${size * 0.72} ${size * 0.35}`}
            stroke="currentColor"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeLinejoin="round"
            fill="none"
            initial={{ pathLength: 0 }}
            animate={{ pathLength: 1 }}
            transition={{ duration: 0.3, delay: 0.1 }}
          />
        </svg>
      </motion.div>
    );
  }

  if (status === 'error') {
    return (
      <motion.div
        initial={{ scale: 0.8 }}
        animate={{ scale: 1 }}
        className={cn('text-error', className)}
      >
        <svg
          width={size}
          height={size}
          viewBox={`0 0 ${size} ${size}`}
          fill="none"
        >
          {/* Background circle */}
          <circle
            cx={center}
            cy={center}
            r={radius}
            fill="currentColor"
            opacity={0.2}
          />
          {/* X mark */}
          <path
            d={`M${size * 0.35} ${size * 0.35} L${size * 0.65} ${size * 0.65}`}
            stroke="currentColor"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
          />
          <path
            d={`M${size * 0.65} ${size * 0.35} L${size * 0.35} ${size * 0.65}`}
            stroke="currentColor"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
          />
        </svg>
      </motion.div>
    );
  }

  if (status === 'cancelled') {
    return (
      <div className={cn('text-text-tertiary', className)}>
        <svg
          width={size}
          height={size}
          viewBox={`0 0 ${size} ${size}`}
          fill="none"
        >
          {/* Background circle */}
          <circle
            cx={center}
            cy={center}
            r={radius}
            stroke="currentColor"
            strokeWidth={strokeWidth}
            opacity={0.3}
          />
          {/* Dash/minus */}
          <path
            d={`M${size * 0.3} ${center} L${size * 0.7} ${center}`}
            stroke="currentColor"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            opacity={0.5}
          />
        </svg>
      </div>
    );
  }

  if (status === 'pending') {
    return (
      <div className={cn('text-text-tertiary', className)}>
        <svg
          width={size}
          height={size}
          viewBox={`0 0 ${size} ${size}`}
          fill="none"
        >
          {/* Background circle */}
          <circle
            cx={center}
            cy={center}
            r={radius}
            stroke="currentColor"
            strokeWidth={strokeWidth}
            opacity={0.3}
          />
        </svg>
      </div>
    );
  }

  if (status === 'transcribing') {
    return (
      <div className={cn('progress-ring-transcribing', className)}>
        <svg
          width={size}
          height={size}
          viewBox={`0 0 ${size} ${size}`}
          fill="none"
          className="animate-spin"
          style={{ animationDuration: '2s' }}
        >
          {/* Background track */}
          <circle
            cx={center}
            cy={center}
            r={radius}
            stroke="rgba(168, 85, 247, 0.2)"
            strokeWidth={strokeWidth}
          />
          {/* Progress arc - purple for transcribing */}
          <circle
            cx={center}
            cy={center}
            r={radius}
            stroke="#a855f7"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={circumference}
            strokeDashoffset={circumference * 0.7}
            transform={`rotate(-90 ${center} ${center})`}
            className="progress-ring-glow-purple"
          />
        </svg>
      </div>
    );
  }

  // Downloading state - default
  return (
    <div className={cn('progress-ring-glow', className)}>
      <svg
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        fill="none"
      >
        {/* Background track */}
        <circle
          cx={center}
          cy={center}
          r={radius}
          stroke="rgba(239, 68, 68, 0.2)"
          strokeWidth={strokeWidth}
        />
        {/* Progress arc */}
        <motion.circle
          cx={center}
          cy={center}
          r={radius}
          stroke="#ef4444"
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          strokeDasharray={circumference}
          initial={{ strokeDashoffset: circumference }}
          animate={{ strokeDashoffset }}
          transition={{ duration: 0.3, ease: 'easeOut' }}
          transform={`rotate(-90 ${center} ${center})`}
        />
      </svg>
    </div>
  );
}
