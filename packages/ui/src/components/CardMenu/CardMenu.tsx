import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type MouseEvent } from 'react';
import { createPortal } from 'react-dom';
import { Icon, type IconName } from '../Icon';

export interface CardMenuItem {
  icon: IconName;
  label: string;
  onSelect: () => void;
  /** Acción destructiva: se pinta con el color de error. */
  danger?: boolean;
}

export interface CardMenuProps {
  /** aria-label del botón de 3 puntos. */
  label: string;
  items: CardMenuItem[];
  /** Tamaño del botón en px (por defecto 24). */
  buttonSize?: number;
  style?: CSSProperties;
}

const menuWrapperStyles: CSSProperties = {
  position: 'relative',
  flexShrink: 0,
};

const menuButtonStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  padding: 0,
  backgroundColor: 'transparent',
  border: 'none',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  flexShrink: 0,
};

// Rendered in a portal with fixed positioning so it isn't clipped by the card's
// overflow:hidden (needed for the thumbnail's rounded corners).
const menuDropdownStyles: CSSProperties = {
  position: 'fixed',
  zIndex: 1000,
  minWidth: '180px',
  display: 'flex',
  flexDirection: 'column',
  padding: '4px',
  backgroundColor: 'var(--lumik-surface-container-high, #2a2929)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  boxShadow: 'var(--lumik-shadow-md, 0 4px 12px rgba(0, 0, 0, 0.4))',
};

const menuItemStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '10px',
  width: '100%',
  padding: '8px 10px',
  background: 'transparent',
  border: 'none',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  textAlign: 'left',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const menuItemDangerStyles: CSSProperties = {
  ...menuItemStyles,
  color: 'var(--lumik-error, #ffb4ab)',
};

/** Botón de 3 puntos con su dropdown, para el header de una card. Los clics no
 *  se propagan a la card (que suele ser clickeable). Sin items no se renderiza. */
export function CardMenu({ label, items, buttonSize = 24, style }: CardMenuProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuPos, setMenuPos] = useState<{ top: number; right: number }>({ top: 0, right: 0 });
  const menuButtonRef = useRef<HTMLButtonElement>(null);
  const menuDropdownRef = useRef<HTMLDivElement>(null);

  // Position the portal dropdown just under the button, right-aligned.
  useLayoutEffect(() => {
    if (!menuOpen || !menuButtonRef.current) return;
    const rect = menuButtonRef.current.getBoundingClientRect();
    setMenuPos({ top: rect.bottom + 4, right: window.innerWidth - rect.right });
  }, [menuOpen]);

  // Close the menu on outside click, Escape, or any scroll/resize (the fixed
  // dropdown would otherwise detach from the button).
  useEffect(() => {
    if (!menuOpen) return;
    const handlePointer = (e: globalThis.MouseEvent) => {
      const target = e.target as Node;
      if (
        !menuButtonRef.current?.contains(target) &&
        !menuDropdownRef.current?.contains(target)
      ) {
        setMenuOpen(false);
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        // Que Escape no cierre también el visor u otros atajos globales.
        e.stopPropagation();
        setMenuOpen(false);
      }
    };
    const handleReflow = () => setMenuOpen(false);
    document.addEventListener('mousedown', handlePointer);
    document.addEventListener('keydown', handleKey, true);
    window.addEventListener('scroll', handleReflow, true);
    window.addEventListener('resize', handleReflow);
    return () => {
      document.removeEventListener('mousedown', handlePointer);
      document.removeEventListener('keydown', handleKey, true);
      window.removeEventListener('scroll', handleReflow, true);
      window.removeEventListener('resize', handleReflow);
    };
  }, [menuOpen]);

  if (items.length === 0) return null;

  const handleMenuToggle = (e: MouseEvent) => {
    e.stopPropagation();
    setMenuOpen((v) => !v);
  };

  const runAction = (e: MouseEvent, action: () => void) => {
    e.stopPropagation();
    setMenuOpen(false);
    action();
  };

  return (
    <div style={{ ...menuWrapperStyles, ...style }}>
      <button
        ref={menuButtonRef}
        style={{ ...menuButtonStyles, width: buttonSize, height: buttonSize }}
        onClick={handleMenuToggle}
        // La card escucha Enter/Espacio para abrirse: no dejar que lo vea.
        onKeyDown={(e) => e.stopPropagation()}
        aria-label={label}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
      >
        <Icon name="menu" size="md" />
      </button>
      {menuOpen && createPortal(
        <div
          ref={menuDropdownRef}
          style={{ ...menuDropdownStyles, top: menuPos.top, right: menuPos.right }}
          role="menu"
          onClick={(e) => e.stopPropagation()}
        >
          {items.map((item) => (
            <button
              key={item.label}
              style={item.danger ? menuItemDangerStyles : menuItemStyles}
              role="menuitem"
              onClick={(e) => runAction(e, item.onSelect)}
            >
              <Icon name={item.icon} size="sm" />
              {item.label}
            </button>
          ))}
        </div>,
        document.body
      )}
    </div>
  );
}
