'use client';

import Image from 'next/image';
import { useState, useCallback, CSSProperties } from 'react';

interface OptimizedImageProps {
  src: string;
  alt: string;
  width?: number | string;
  height?: number | string;
  blurColor?: string;
  lazy?: boolean;
  showSkeleton?: boolean;
  className?: string;
  style?: CSSProperties;
  priority?: boolean;
  sizes?: string;
  quality?: number;
}

export default function OptimizedImage({
  src,
  alt,
  width,
  height,
  blurColor,
  lazy = true,
  showSkeleton = true,
  className = '',
  style,
  priority = false,
  sizes,
  quality,
}: OptimizedImageProps) {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(false);

  const handleLoad = useCallback(() => {
    setIsLoading(false);
  }, []);

  const handleError = useCallback(() => {
    setIsLoading(false);
    setError(true);
  }, []);

  const generatePlaceholderColor = (input: string): string => {
    let hash = 0;
    for (let i = 0; i < input.length; i++) {
      hash = input.charCodeAt(i) + ((hash << 5) - hash);
    }
    const hue = Math.abs(hash % 360);
    return `hsl(${hue}, 30%, 90%)`;
  };

  const placeholderColor = blurColor || generatePlaceholderColor(String(src));

  const objectFit = className.includes('object-cover') ? 'cover' : 
                    className.includes('object-contain') ? 'contain' : 'fill';

  const containerStyle: CSSProperties = {
    width: width,
    height: height,
    backgroundColor: isLoading ? placeholderColor : 'transparent',
    transition: 'background-color 0.3s ease',
    position: 'relative',
    overflow: 'hidden',
    ...style,
  };

  if (error) {
    return (
      <div
        className={`flex items-center justify-center bg-gray-100 dark:bg-gray-800 ${className}`}
        style={containerStyle}
        role="img"
        aria-label={`Failed to load: ${alt}`}
      >
        <span className="text-gray-400 text-sm">Failed to load image</span>
      </div>
    );
  }

  return (
    <div className={className} style={containerStyle}>
      {showSkeleton && isLoading && (
        <div
          className="absolute inset-0 animate-pulse"
          style={{
            background: `linear-gradient(90deg, ${placeholderColor} 0%, #e5e7eb 50%, ${placeholderColor} 100%)`,
            backgroundSize: '200% 100%',
          }}
        />
      )}

      <Image
        src={src}
        alt={alt}
        width={typeof width === 'number' ? width : undefined}
        height={typeof height === 'number' ? height : undefined}
        loading={lazy ? 'lazy' : 'eager'}
        onLoad={handleLoad}
        onError={handleError}
        placeholder="empty"
        priority={priority}
        sizes={sizes}
        quality={quality}
        style={{
          objectFit,
          opacity: isLoading ? 0 : 1,
          transition: 'opacity 0.3s ease-in-out',
          width: '100%',
          height: '100%',
        }}
      />
    </div>
  );
}

interface AvatarProps {
  src?: string;
  alt: string;
  size?: number;
}

export function Avatar({ src, alt, size = 40 }: AvatarProps) {
  const [imageError, setImageError] = useState(false);

  const handleError = useCallback(() => {
    setImageError(true);
  }, []);

  const getInitials = (name: string): string => {
    return name
      .split(' ')
      .map(word => word[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  if (!src || imageError) {
    return (
      <div
        className="rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white font-medium"
        style={{ width: size, height: size, fontSize: size * 0.4 }}
        role="img"
        aria-label={alt}
      >
        {getInitials(alt || 'U')}
      </div>
    );
  }

  return (
    <Image
      src={src}
      alt={alt}
      width={size}
      height={size}
      onError={handleError}
      style={{
        borderRadius: '50%',
        objectFit: 'cover',
      }}
    />
  );
}

export const imageSizes = {
  grid: '(max-width: 640px) 100vw, (max-width: 768px) 50vw, (max-width: 1024px) 33vw, 25vw',
  hero: '100vw',
  card: '(max-width: 640px) 100vw, (max-width: 768px) 50vw, 33vw',
  avatar: '40px',
  thumbnail: '(max-width: 640px) 50vw, (max-width: 768px) 33vw, 25vw',
} as const;
