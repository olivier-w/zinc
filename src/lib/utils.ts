export function formatDuration(seconds: number | null): string {
  if (seconds === null) return '--:--';

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

export function formatBytes(bytes: number | null): string {
  if (bytes === null || bytes === 0) return '0 B';

  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function formatViewCount(count: number | null): string {
  if (count === null) return '';

  if (count >= 1_000_000_000) {
    return `${(count / 1_000_000_000).toFixed(1)}B views`;
  }
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M views`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}K views`;
  }
  return `${count} views`;
}

export function formatDate(dateStr: string | null): string {
  if (!dateStr) return '';

  // yt-dlp returns dates as YYYYMMDD
  if (dateStr.length === 8) {
    const year = dateStr.slice(0, 4);
    const month = dateStr.slice(4, 6);
    const day = dateStr.slice(6, 8);
    const date = new Date(`${year}-${month}-${day}`);
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  }

  return dateStr;
}

export function isValidUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return ['http:', 'https:'].includes(parsed.protocol);
  } catch {
    return false;
  }
}

export function isVideoUrl(url: string): boolean {
  if (!isValidUrl(url)) return false;

  const videoHosts = [
    'youtube.com',
    'youtu.be',
    'vimeo.com',
    'dailymotion.com',
    'twitch.tv',
    'twitter.com',
    'x.com',
    'instagram.com',
    'facebook.com',
    'tiktok.com',
    'reddit.com',
    'soundcloud.com',
    'bandcamp.com',
  ];

  try {
    const parsed = new URL(url);
    return videoHosts.some(host =>
      parsed.hostname === host || parsed.hostname.endsWith(`.${host}`)
    );
  } catch {
    return false;
  }
}

export function cn(...classes: (string | boolean | undefined | null)[]): string {
  return classes.filter(Boolean).join(' ');
}

export function truncate(str: string, length: number): string {
  if (str.length <= length) return str;
  return str.slice(0, length - 1) + '\u2026';
}
