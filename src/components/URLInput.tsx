import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { motion } from 'motion/react';
import { isValidUrl } from '@/lib/utils';
import { DownloadIcon, LoaderIcon, AlertCircleIcon } from './Icons';

interface URLInputProps {
  onSubmit: (url: string) => void;
  isLoading?: boolean;
  disabled?: boolean;
  variant?: 'hero' | 'compact';
}

export function URLInput({ onSubmit, isLoading = false, disabled = false, variant: _variant = 'hero' }: URLInputProps) {
  const [url, setUrl] = useState('');
  const [shake, setShake] = useState(false);
  const [isFocused, setIsFocused] = useState(false);
  const [isDragOver, setIsDragOver] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Derive validation state from url
  const isValid = useMemo(() => {
    if (url.trim() === '') {
      return null;
    }
    return isValidUrl(url.trim());
  }, [url]);

  // Auto-focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Check clipboard on window focus
  useEffect(() => {
    async function checkClipboard() {
      try {
        const text = await navigator.clipboard.readText();
        if (text && isValidUrl(text.trim()) && url === '') {
          setUrl(text.trim());
        }
      } catch {
        // Clipboard access denied, ignore
      }
    }

    window.addEventListener('focus', checkClipboard);
    return () => window.removeEventListener('focus', checkClipboard);
  }, [url]);

  const handleSubmit = useCallback((e?: React.FormEvent) => {
    e?.preventDefault();
    const trimmed = url.trim();

    if (!isValidUrl(trimmed)) {
      setShake(true);
      setTimeout(() => setShake(false), 500);
      return;
    }

    onSubmit(trimmed);
    setUrl('');
  }, [url, onSubmit]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }, [handleSubmit]);

  // Handle drag and drop
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
  }, []);

  const handleDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    // Only set false if leaving the container (not entering a child)
    if (!e.currentTarget.contains(e.relatedTarget as Node)) {
      setIsDragOver(false);
    }
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    const text = e.dataTransfer.getData('text/plain');
    if (text && isValidUrl(text.trim())) {
      setUrl(text.trim());
    }
  }, []);

  const handlePaste = useCallback((e: React.ClipboardEvent) => {
    const text = e.clipboardData.getData('text/plain').trim();
    if (text && isValidUrl(text)) {
      e.preventDefault();
      setUrl(text);
      // Auto-submit after a brief delay to show the URL
      setTimeout(() => onSubmit(text), 100);
    }
  }, [onSubmit]);

  return (
    <form onSubmit={handleSubmit} className={`w-full transition-all duration-300 max-w-xl`}>
      <div
        className={`
          relative flex items-center gap-2 rounded-full
          glass transition-all duration-300 ease-out p-2
          ${shake ? 'shake' : ''}
          ${isLoading ? 'glow-loading' : ''}
          ${!isLoading && isValid === true ? 'glow-success' : ''}
          ${isValid === false ? 'border-error/50' : ''}
          ${!isLoading && isFocused ? 'glow-focus' : ''}
          ${isDragOver ? 'border-accent ring-2 ring-accent/30' : ''}
        `}
        onDragOver={handleDragOver}
        onDragEnter={handleDragEnter}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <input
          ref={inputRef}
          type="url"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          placeholder="Paste a video URL"
          disabled={disabled || isLoading}
          className={`
            flex-1 bg-transparent text-text-primary placeholder-text-tertiary
            outline-none focus:outline-none focus-visible:outline-none
            disabled:opacity-50 disabled:cursor-not-allowed
            transition-all duration-300 px-3 py-2 text-base
          `}
          aria-label="Video URL"
          aria-invalid={isValid === false}
          aria-describedby={isValid === false ? 'url-error' : undefined}
        />

        <button
          type="submit"
          disabled={disabled || isLoading || !isValid}
          className={`
            rounded-full btn-gradient text-white
            flex items-center justify-center transition-all duration-300
            disabled:opacity-50 disabled:cursor-not-allowed p-3 min-w-[44px]
          `}
          aria-label={isLoading ? 'Loading video info' : 'Get video info'}
        >
          {isLoading ? (
            <LoaderIcon className={`animate-spin transition-all duration-300 w-5 h-5`} />
          ) : (
            <DownloadIcon className={`transition-all duration-300 w-5 h-5`} />
          )}
        </button>
      </div>

      {isValid === false && url.trim() !== '' && (
        <motion.div
          id="url-error"
          role="alert"
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
          className="mt-3 flex items-center justify-center gap-2 px-4 py-2 rounded-full bg-error/10 border border-error/20"
        >
          <AlertCircleIcon className="w-4 h-4 text-error shrink-0" />
          <span className="text-sm text-error/90">Please enter a valid URL</span>
        </motion.div>
      )}
    </form>
  );
}
