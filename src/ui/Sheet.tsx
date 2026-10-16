// SPDX-License-Identifier: GPL-3.0-or-later
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { IconClose } from "./icons";

interface SheetControls {
  /** Close with the exit animation, then call the sheet's `onClose`. */
  close: () => void;
}

const SheetContext = createContext<SheetControls>({ close: () => {} });

/** Lets a button inside a sheet (Cancel, Done) close it with the exit animation. */
export function useSheet(): SheetControls {
  return useContext(SheetContext);
}

const FOCUSABLE =
  'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

/**
 * A modal sheet: dims the window, slides up and scales in, closes on Escape or
 * a click outside, keeps keyboard focus inside while open and gives it back
 * afterwards. `dismissible={false}` pins it while work is running.
 */
export function Sheet({
  title,
  subtitle,
  onClose,
  footer,
  children,
  dismissible = true,
  width = 480,
}: {
  title: string;
  subtitle?: string;
  onClose: () => void;
  footer?: ReactNode;
  children: ReactNode;
  dismissible?: boolean;
  width?: number;
}) {
  const [closing, setClosing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  });

  const close = useCallback(() => setClosing(true), []);
  const controls = useMemo<SheetControls>(() => ({ close }), [close]);

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const first = panel?.querySelector<HTMLElement>("[data-autofocus]") ?? panel?.querySelector<HTMLElement>(FOCUSABLE);
    (first ?? panel)?.focus({ preventScroll: true });
    return () => previous?.focus?.({ preventScroll: true });
  }, []);

  // Listen on the document, not the panel: when a step changes, the button
  // that had focus is removed and focus falls back to the page - Escape and
  // Tab must keep working from there.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && dismissible) {
        event.preventDefault();
        event.stopPropagation();
        close();
        return;
      }
      if (event.key !== "Tab") return;
      const panel = panelRef.current;
      if (!panel) return;
      const nodes = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE));
      if (nodes.length === 0) return;
      const first = nodes[0];
      const last = nodes[nodes.length - 1];
      const active = document.activeElement;
      if (!panel.contains(active)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (event.shiftKey && active === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && active === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, [dismissible, close]);

  return (
    <SheetContext.Provider value={controls}>
      <div
        className={`sheet-overlay${closing ? " is-closing" : ""}`}
        onMouseDown={(event) => {
          if (dismissible && event.target === event.currentTarget) close();
        }}
        onAnimationEnd={(event) => {
          if (closing && event.target === event.currentTarget) onCloseRef.current();
        }}
      >
        <div
          ref={panelRef}
          className="sheet"
          style={{ width: `min(${width}px, calc(100vw - 32px))` }}
          role="dialog"
          aria-modal="true"
          aria-label={title}
          tabIndex={-1}
        >
          <header className="sheet-header">
            <div className="sheet-heading">
              <h2>{title}</h2>
              {subtitle && <p>{subtitle}</p>}
            </div>
            {dismissible && (
              <button type="button" className="btn btn-icon" aria-label="Close" onClick={close}>
                <IconClose />
              </button>
            )}
          </header>
          <div className="sheet-body">{children}</div>
          {footer && <footer className="sheet-footer">{footer}</footer>}
        </div>
      </div>
    </SheetContext.Provider>
  );
}

/** The footer's quiet button: closes the sheet with its exit animation. */
export function SheetClose({
  children = "Cancel",
  disabled,
  primary,
}: {
  children?: ReactNode;
  disabled?: boolean;
  primary?: boolean;
}) {
  const { close } = useSheet();
  return (
    <button type="button" className={primary ? "btn btn-primary" : "btn"} disabled={disabled} onClick={close}>
      {children}
    </button>
  );
}
