// Reordenar un álbum arrastrando (spec `photo-albums`, design D3 de
// `album-custom-order`). Con eventos de puntero, no con el drag de HTML5: con
// `dragDropEnabled` de Tauri, WebView2 no lo entrega dentro de la página. El
// destino sale de la geometría del grid (columnas y alto de fila), no del DOM,
// porque el grid es virtual y la fila de destino puede no estar montada.
import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';

/** Distancia (px) que hay que mover el puntero para que un clic pase a ser arrastre. */
const DRAG_THRESHOLD = 6;
/** Franja (px) junto al borde del grid que lo desplaza al arrastrar. */
const SCROLL_EDGE = 64;
/** Desplazamiento máximo por cuadro (px), pegado al borde. */
const SCROLL_MAX_STEP = 18;

export interface AlbumDrag {
  itemId: string;
  /** Lugar donde caería: índice en la lista sin la foto arrastrada. */
  to: number;
  /** Posición del puntero en la ventana, para la miniatura que lo sigue. */
  x: number;
  y: number;
}

interface Grid {
  scrollEl: HTMLDivElement | null;
  cols: number;
  cardWidth: number;
  colGap: number;
  rowHeight: number;
  /** Elementos del álbum. */
  count: number;
}

interface Options extends Grid {
  /** Solo en el orden personalizado. */
  enabled: boolean;
  onDrop: (itemId: string, to: number) => void;
}

/** Celda del grid bajo el punto (`x`, `y` relativos al contenido del grid, con su scroll). */
export function cellAt(x: number, y: number, { cols, cardWidth, colGap, rowHeight, count }: Omit<Grid, 'scrollEl'>): number {
  const col = Math.min(cols - 1, Math.max(0, Math.floor((x + colGap / 2) / (cardWidth + colGap))));
  const row = Math.max(0, Math.floor(y / rowHeight));
  return Math.min(count - 1, Math.max(0, row * cols + col));
}

export function useAlbumDrag({ enabled, onDrop, ...grid }: Options) {
  const [drag, setDrag] = useState<AlbumDrag | null>(null);
  // Lo que leen los escuchas de la ventana y el bucle de auto-scroll, sin depender de renders.
  const live = useRef<{ itemId: string; from: number; startX: number; startY: number; active: boolean; x: number; y: number; to: number } | null>(null);
  const gridRef = useRef(grid);
  gridRef.current = grid;
  const onDropRef = useRef(onDrop);
  onDropRef.current = onDrop;
  const cleanup = useRef<(() => void) | null>(null);

  // Al desmontar (p. ej. al salir del álbum) no quedan escuchas colgados.
  useEffect(() => () => cleanup.current?.(), []);

  const startDrag = useCallback(
    (e: ReactPointerEvent, itemId: string, index: number) => {
      // El menú y las estrellas de la tarjeta son botones: ahí no se arrastra.
      if (!enabled || e.button !== 0 || (e.target as HTMLElement).closest('button')) return;
      cleanup.current?.();
      live.current = { itemId, from: index, startX: e.clientX, startY: e.clientY, active: false, x: e.clientX, y: e.clientY, to: index };
      let frame = 0;

      const target = () => {
        const d = live.current;
        const g = gridRef.current;
        if (!d || !g.scrollEl) return d?.to ?? 0;
        const rect = g.scrollEl.getBoundingClientRect();
        return cellAt(d.x - rect.left, d.y - rect.top + g.scrollEl.scrollTop, g);
      };
      const publish = () => {
        const d = live.current;
        if (d?.active) setDrag({ itemId: d.itemId, to: d.to, x: d.x, y: d.y });
      };
      // Junto al borde, el grid se desplaza; el destino se recalcula con la última posición conocida.
      const autoScroll = () => {
        const d = live.current;
        const el = gridRef.current.scrollEl;
        if (!d?.active || !el) return;
        const rect = el.getBoundingClientRect();
        const up = rect.top + SCROLL_EDGE - d.y;
        const down = d.y - (rect.bottom - SCROLL_EDGE);
        const step = up > 0 ? -Math.min(1, up / SCROLL_EDGE) : down > 0 ? Math.min(1, down / SCROLL_EDGE) : 0;
        if (step !== 0) {
          el.scrollTop += step * SCROLL_MAX_STEP;
          const to = target();
          if (to !== d.to) {
            d.to = to;
            publish();
          }
        }
        frame = requestAnimationFrame(autoScroll);
      };

      const onMove = (ev: PointerEvent) => {
        const d = live.current;
        if (!d) return;
        d.x = ev.clientX;
        d.y = ev.clientY;
        if (!d.active) {
          if (Math.hypot(d.x - d.startX, d.y - d.startY) < DRAG_THRESHOLD) return;
          d.active = true;
          document.body.style.userSelect = 'none';
          frame = requestAnimationFrame(autoScroll);
        }
        d.to = target();
        publish();
      };
      const finish = (drop: boolean, ev?: PointerEvent) => {
        const d = live.current;
        cleanup.current?.();
        if (!d?.active) return;
        // El clic que el navegador manda al soltar no abre el visor.
        const swallow = (c: MouseEvent) => {
          c.stopPropagation();
          c.preventDefault();
        };
        window.addEventListener('click', swallow, { capture: true, once: true });
        setTimeout(() => window.removeEventListener('click', swallow, { capture: true }), 0);
        // Soltar fuera del grid cancela.
        const el = gridRef.current.scrollEl;
        const inside = ev && el && (() => {
          const r = el.getBoundingClientRect();
          return ev.clientX >= r.left && ev.clientX <= r.right && ev.clientY >= r.top && ev.clientY <= r.bottom;
        })();
        if (drop && inside && d.to !== d.from) onDropRef.current(d.itemId, d.to);
      };
      const onUp = (ev: PointerEvent) => finish(true, ev);
      const onCancel = () => finish(false);
      const onKey = (ev: KeyboardEvent) => {
        if (ev.key === 'Escape' && live.current?.active) {
          ev.stopPropagation();
          finish(false);
        }
      };

      window.addEventListener('pointermove', onMove);
      window.addEventListener('pointerup', onUp);
      window.addEventListener('pointercancel', onCancel);
      window.addEventListener('keydown', onKey, true);
      cleanup.current = () => {
        window.removeEventListener('pointermove', onMove);
        window.removeEventListener('pointerup', onUp);
        window.removeEventListener('pointercancel', onCancel);
        window.removeEventListener('keydown', onKey, true);
        cancelAnimationFrame(frame);
        document.body.style.userSelect = '';
        live.current = null;
        cleanup.current = null;
        setDrag(null);
      };
    },
    [enabled],
  );

  return { drag, startDrag };
}
