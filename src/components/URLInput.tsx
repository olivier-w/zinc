import { useState, useCallback, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import { isValidUrl } from '@/lib/utils';
import { ClipboardIcon, DownloadIcon, LoaderIcon } from './Icons';

interface URLInputProps {
  onSubmit: (url: string) => void;
  isLoading?: boolean;
  disabled?: boolean;
}

export function URLInput({ onSubmit, isLoading = false, disabled = false }: URLInputProps) {
  const [url, setUrl] = useState('');
  const [isValid, setIsValid] = useState<boolean | null>(null);
  const [shake, setShake] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Validate URL on change
  useEffect(() => {
    if (url.trim() === '') {
      setIsValid(null);
    } else {
      setIsValid(isValidUrl(url.trim()));
    }
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

  const handlePaste = useCallback(async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setUrl(text.trim());
      }
    } catch {
      // Clipboard access denied
    }
  }, []);

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
    setIsValid(null);
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

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const text = e.dataTransfer.getData('text/plain');
    if (text && isValidUrl(text.trim())) {
      setUrl(text.trim());
    }
  }, []);

  return (
    <form onSubmit={handleSubmit} className="w-full max-w-2xl">
      <motion.div
        className={`
          relative flex items-center gap-2 p-2 rounded-xl
          bg-bg-secondary border transition-all duration-200
          ${shake ? 'shake' : ''}
          ${isValid === true ? 'border-success/50 shadow-[0_0_20px_rgba(34,197,94,0.15)]' : ''}
          ${isValid === false ? 'border-error/50' : ''}
          ${isValid === null ? 'border-border hover:border-border-hover' : ''}
          focus-within:border-accent focus-within:shadow-glow
        `}
        whileFocus={{ scale: 1.01 }}
        onDragOver={handleDragOver}
        onDrop={handleDrop}
      >
        <input
          ref={inputRef}
          type="url"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Paste a video URL (YouTube, Vimeo, Twitter...)"
          disabled={disabled || isLoading}
          className={`
            flex-1 bg-transparent text-text-primary placeholder-text-tertiary
            px-4 py-3 text-base outline-none
            disabled:opacity-50 disabled:cursor-not-allowed
          `}
          aria-label="Video URL"
          aria-invalid={isValid === false}
          aria-describedby={isValid === false ? 'url-error' : undefined}
        />

        <button
          type="button"
          onClick={handlePaste}
          disabled={disabled || isLoading}
          className={`
            p-3 rounded-lg text-text-secondary hover:text-text-primary
            hover:bg-bg-tertiary transition-colors
            disabled:opacity-50 disabled:cursor-not-allowed
          `}
          aria-label="Paste from clipboard"
        >
          <ClipboardIcon className="w-5 h-5" />
        </button>

        <motion.button
          type="submit"
          disabled={disabled || isLoading || !isValid}
          className={`
            p-3 rounded-lg bg-accent text-white
            hover:bg-accent-hover transition-colors
            disabled:opacity-50 disabled:cursor-not-allowed
            flex items-center justify-center min-w-[48px]
          `}
          whileTap={{ scale: 0.98 }}
          aria-label={isLoading ? 'Loading video info' : 'Get video info'}
        >
          {isLoading ? (
            <LoaderIcon className="w-5 h-5 animate-spin" />
          ) : (
            <DownloadIcon className="w-5 h-5" />
          )}
        </motion.button>
      </motion.div>

      {isValid === false && url.trim() !== '' && (
        <p id="url-error" className="mt-2 text-sm text-error" role="alert">
          Please enter a valid URL
        </p>
      )}
    </form>
  );
}
