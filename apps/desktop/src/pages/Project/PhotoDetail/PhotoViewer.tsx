import {
  useState,
  useEffect,
  useRef,
  useCallback,
  forwardRef,
  useImperativeHandle,
  type CSSProperties,
} from 'react';
import { PhotoViewerToolbar } from './PhotoViewerToolbar';

export interface HistogramBins {
  r: Uint32Array;
  g: Uint32Array;
  b: Uint32Array;
}

export interface PhotoViewerHandle {
  zoomIn(): void;
  zoomOut(): void;
  fitToScreen(): void;
  rotateLeft(): void;
  rotateRight(): void;
}

export interface PhotoViewerProps {
  photoId: string;
  fullImageUrl: string | null;
  fullImageLoading: boolean;
  initialRotation: number;
  hasPrev: boolean;
  hasNext: boolean;
  onPrev: () => void;
  onNext: () => void;
  onRotationChange: (rotation: number) => void;
  onHistogramReady?: (bins: HistogramBins) => void;
}

const viewerRootStyle: CSSProperties = {
  flex: 1,
  display: 'flex',
  flexDirection: 'column',
  overflow: 'hidden',
  minWidth: 0,
};

const canvasWrapStyle: CSSProperties = {
  flex: 1,
  position: 'relative',
  overflow: 'hidden',
  background: 'var(--lumik-surface-container-lowest, #0e0e0e)',
};

const loadingOverlayStyle: CSSProperties = {
  position: 'absolute',
  bottom: '12px',
  right: '12px',
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  padding: '4px 10px',
  borderRadius: 'var(--lumik-radius, 4px)',
  background: 'rgba(14, 14, 14, 0.8)',
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '11px',
  color: 'var(--lumik-outline, #8c90a0)',
  backdropFilter: 'blur(4px)',
  pointerEvents: 'none',
};

export const PhotoViewer = forwardRef<PhotoViewerHandle, PhotoViewerProps>(
  (
    {
      photoId,
      fullImageUrl,
      fullImageLoading,
      initialRotation,
      hasPrev,
      hasNext,
      onPrev,
      onNext,
      onRotationChange,
      onHistogramReady,
    },
    ref,
  ) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const imgRef = useRef<HTMLImageElement | null>(null);

    // All transform state in refs — canvas redraws via RAF, no React re-renders
    const scaleRef = useRef(1);
    const offsetRef = useRef({ x: 0, y: 0 });
    const rotationRef = useRef(0);
    const isDraggingRef = useRef(false);
    const dragRef = useRef({ startX: 0, startY: 0, startOX: 0, startOY: 0 });
    const rafRef = useRef<number | null>(null);

    // React state only for toolbar display values
    const [displayScale, setDisplayScale] = useState(100);
    const [isDragging, setIsDragging] = useState(false);
    const workerRef = useRef<Worker | null>(null);

    // Create histogram worker once; terminate on unmount
    useEffect(() => {
      const worker = new Worker(
        new URL('../../../workers/histogram.worker.ts', import.meta.url),
        { type: 'module' },
      );
      worker.onmessage = (e: MessageEvent<HistogramBins>) => onHistogramReady?.(e.data);
      workerRef.current = worker;
      return () => worker.terminate();
    }, []); // eslint-disable-line react-hooks/exhaustive-deps

    // Stable draw — reads all values from refs at call time (empty deps)
    const redraw = useCallback(() => {
      const c = canvasRef.current;
      const img = imgRef.current;
      if (!c || !img) return;
      const ctx = c.getContext('2d');
      if (!ctx) return;
      const { width, height } = c;
      ctx.clearRect(0, 0, width, height);
      ctx.save();
      ctx.translate(width / 2 + offsetRef.current.x, height / 2 + offsetRef.current.y);
      ctx.rotate((rotationRef.current * Math.PI) / 180);
      ctx.scale(scaleRef.current, scaleRef.current);
      ctx.drawImage(img, -img.naturalWidth / 2, -img.naturalHeight / 2);
      ctx.restore();
    }, []);

    const scheduleRedraw = useCallback(() => {
      if (rafRef.current !== null) return;
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = null;
        redraw();
      });
    }, [redraw]);

    const fitToScreen = useCallback(() => {
      const c = canvasRef.current;
      const img = imgRef.current;
      if (!c || !img || c.width === 0 || c.height === 0 || img.naturalWidth === 0) return;
      const rot = rotationRef.current;
      const imgW = rot % 180 === 0 ? img.naturalWidth : img.naturalHeight;
      const imgH = rot % 180 === 0 ? img.naturalHeight : img.naturalWidth;
      const newScale = Math.min((c.width * 0.97) / imgW, (c.height * 0.97) / imgH);
      scaleRef.current = newScale;
      offsetRef.current = { x: 0, y: 0 };
      setDisplayScale(Math.round(newScale * 100));
      scheduleRedraw();
    }, [scheduleRedraw]);

    const rotateLeft = useCallback(() => {
      rotationRef.current = (rotationRef.current - 90 + 360) % 360;
      fitToScreen();
      onRotationChange(rotationRef.current);
    }, [fitToScreen, onRotationChange]);

    const rotateRight = useCallback(() => {
      rotationRef.current = (rotationRef.current + 90) % 360;
      fitToScreen();
      onRotationChange(rotationRef.current);
    }, [fitToScreen, onRotationChange]);

    const zoomIn = useCallback(() => {
      scaleRef.current = Math.min(scaleRef.current * 1.25, 30);
      setDisplayScale(Math.round(scaleRef.current * 100));
      scheduleRedraw();
    }, [scheduleRedraw]);

    const zoomOut = useCallback(() => {
      scaleRef.current = Math.max(scaleRef.current / 1.25, 0.05);
      setDisplayScale(Math.round(scaleRef.current * 100));
      scheduleRedraw();
    }, [scheduleRedraw]);

    useImperativeHandle(
      ref,
      () => ({ zoomIn, zoomOut, fitToScreen, rotateLeft, rotateRight }),
      [zoomIn, zoomOut, fitToScreen, rotateLeft, rotateRight],
    );

    // Update rotation ref when photo changes or when EXIF rotation arrives.
    // Does NOT redraw — the full-image effect below owns the first draw of each photo.
    useEffect(() => {
      rotationRef.current = initialRotation;
    }, [photoId, initialRotation]);

    // Load full-res preview — single draw point for the canvas.
    useEffect(() => {
      if (!fullImageUrl) return;
      let stale = false;
      const img = new Image();
      img.onload = () => {
        if (stale) return;
        imgRef.current = img;
        fitToScreen();

        // Sample image at reduced size and send to histogram worker
        const MAX = 512;
        const scale = Math.min(1, MAX / Math.max(img.naturalWidth, img.naturalHeight));
        const w = Math.round(img.naturalWidth * scale);
        const h = Math.round(img.naturalHeight * scale);
        const offscreen = document.createElement('canvas');
        offscreen.width = w;
        offscreen.height = h;
        const ctx2 = offscreen.getContext('2d');
        if (ctx2 && workerRef.current) {
          ctx2.drawImage(img, 0, 0, w, h);
          const imageData = ctx2.getImageData(0, 0, w, h);
          workerRef.current.postMessage({ buffer: imageData.data.buffer }, [imageData.data.buffer]);
        }
      };
      img.src = fullImageUrl;
      return () => { stale = true; };
    }, [fullImageUrl, fitToScreen]);

    // Sync canvas pixel dimensions with CSS size via ResizeObserver
    useEffect(() => {
      const container = containerRef.current;
      const c = canvasRef.current;
      if (!container || !c) return;
      const observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
          const { width, height } = entry.contentRect;
          c.width = Math.round(width);
          c.height = Math.round(height);
          fitToScreen();
        }
      });
      observer.observe(container);
      return () => observer.disconnect();
    }, [fitToScreen]);

    // Mouse wheel zoom centered at cursor
    useEffect(() => {
      const c = canvasRef.current;
      if (!c) return;
      function onWheel(e: WheelEvent) {
        if (!c) return;
        e.preventDefault();
        const rect = c.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;
        const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12;
        const oldScale = scaleRef.current;
        const newScale = Math.min(Math.max(oldScale * factor, 0.05), 30);
        const ratio = newScale / oldScale;
        const cx = c.width / 2;
        const cy = c.height / 2;
        const ox = offsetRef.current.x;
        const oy = offsetRef.current.y;
        offsetRef.current = {
          x: (mouseX - cx) * (1 - ratio) + ox * ratio,
          y: (mouseY - cy) * (1 - ratio) + oy * ratio,
        };
        scaleRef.current = newScale;
        setDisplayScale(Math.round(newScale * 100));
        scheduleRedraw();
      }
      c.addEventListener('wheel', onWheel, { passive: false });
      return () => c.removeEventListener('wheel', onWheel);
    }, [scheduleRedraw]);

    // Mouse drag pan
    const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
      isDraggingRef.current = true;
      setIsDragging(true);
      dragRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        startOX: offsetRef.current.x,
        startOY: offsetRef.current.y,
      };
    }, []);

    const handleMouseMove = useCallback(
      (e: React.MouseEvent<HTMLDivElement>) => {
        if (!isDraggingRef.current) return;
        offsetRef.current = {
          x: dragRef.current.startOX + (e.clientX - dragRef.current.startX),
          y: dragRef.current.startOY + (e.clientY - dragRef.current.startY),
        };
        scheduleRedraw();
      },
      [scheduleRedraw],
    );

    const handleMouseUp = useCallback(() => {
      isDraggingRef.current = false;
      setIsDragging(false);
    }, []);

    return (
      <div style={viewerRootStyle}>
        <div
          ref={containerRef}
          style={{ ...canvasWrapStyle, cursor: isDragging ? 'grabbing' : 'grab' }}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
        >
          <canvas ref={canvasRef} style={{ display: 'block', width: '100%', height: '100%' }} />
          {fullImageLoading && (
            <div style={loadingOverlayStyle}>
              <span
                style={{
                  display: 'inline-block',
                  width: '12px',
                  height: '12px',
                  borderRadius: '50%',
                  border: '2px solid var(--lumik-outline-variant, #424654)',
                  borderTopColor: 'var(--lumik-primary, #b0c6ff)',
                  animation: 'spin 0.8s linear infinite',
                }}
              />
              Cargando preview...
            </div>
          )}
        </div>

        <PhotoViewerToolbar
          displayScale={displayScale}
          hasPrev={hasPrev}
          hasNext={hasNext}
          onPrev={onPrev}
          onNext={onNext}
          onZoomIn={zoomIn}
          onZoomOut={zoomOut}
          onFitToScreen={fitToScreen}
          onRotateLeft={rotateLeft}
          onRotateRight={rotateRight}
        />
      </div>
    );
  },
);
