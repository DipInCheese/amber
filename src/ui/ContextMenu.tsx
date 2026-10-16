// SPDX-License-Identifier: GPL-3.0-or-later
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from "react";

export interface MenuItem {
  label: string;
  icon?: ReactNode;
  onSelect: () => void;
}

interface OpenMenu {
  x: number;
  y: number;
  items: MenuItem[];
}

interface ContextMenuApi {
  /** Open a menu at the pointer for a right-click (or the long-press equivalent). */
  show: (event: ReactMouseEvent, items: MenuItem[]) => void;
}

const ContextMenuContext = createContext<ContextMenuApi>({ show: () => {} });

export function useContextMenu(): ContextMenuApi {
  return useContext(ContextMenuContext);
}

/** Right-click menus in the app's own style, replacing the browser's. */
export function ContextMenuProvider({ children }: { children: ReactNode }) {
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const close = useCallback(() => setMenu(null), []);

  const api = useMemo<ContextMenuApi>(
    () => ({
      show: (event, items) => {
        if (items.length === 0) return;
        event.preventDefault();
        event.stopPropagation();
        setMenu({ x: event.clientX, y: event.clientY, items });
      },
    }),
    [],
  );

  return (
    <ContextMenuContext.Provider value={api}>
      {children}
      {menu && <MenuView menu={menu} onClose={close} />}
    </ContextMenuContext.Provider>
  );
}

function MenuView({ menu, onClose }: { menu: OpenMenu; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: menu.x, top: menu.y });

  // Keep the menu inside the window.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const margin = 8;
    const left = Math.min(menu.x, window.innerWidth - el.offsetWidth - margin);
    const top = Math.min(menu.y, window.innerHeight - el.offsetHeight - margin);
    setPosition({ left: Math.max(margin, left), top: Math.max(margin, top) });
    el.querySelector<HTMLElement>("button")?.focus({ preventScroll: true });
  }, [menu]);

  useEffect(() => {
    const onPointerDown = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      event.preventDefault();
      const buttons = Array.from(ref.current?.querySelectorAll<HTMLElement>("button") ?? []);
      const at = buttons.indexOf(document.activeElement as HTMLElement);
      const step = event.key === "ArrowDown" ? 1 : -1;
      buttons[(at + step + buttons.length) % buttons.length]?.focus();
    };
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("blur", onClose);
    window.addEventListener("resize", onClose);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("resize", onClose);
    };
  }, [onClose]);

  return (
    <div ref={ref} className="menu" role="menu" style={position} onContextMenu={(e) => e.preventDefault()}>
      {menu.items.map((item) => (
        <button
          key={item.label}
          type="button"
          role="menuitem"
          className="menu-item"
          onClick={() => {
            onClose();
            item.onSelect();
          }}
        >
          {item.icon}
          {item.label}
        </button>
      ))}
    </div>
  );
}
