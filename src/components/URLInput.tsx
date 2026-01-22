import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
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

  return (
    <form onSubmit={handleSubmit} className="w-full max-w-3xl">
      <motion.div
        className={`
          relative flex items-center gap-2 p-3 rounded-full
          glass transition-all duration-300
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
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          placeholder="Paste a video URL"
          disabled={disabled || isLoading}
          className={`
            flex-1 bg-transparent text-text-primary placeholder-text-tertiary
            px-4 py-3.5 text-base outline-none focus:outline-none focus-visible:outline-none
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
            p-3 rounded-full text-text-secondary hover:text-text-primary
            hover:bg-white/10 transition-colors
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
            p-3.5 rounded-full btn-gradient text-white
            flex items-center justify-center min-w-[52px]
            disabled:opacity-50 disabled:cursor-not-allowed
          `}
          whileTap={{ scale: 0.95 }}
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
        <p id="url-error" className="mt-2 text-sm text-error text-center" role="alert">
          Please enter a valid URL
        </p>
      )}
    </form>
  );
}
