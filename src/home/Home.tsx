// SPDX-License-Identifier: GPL-3.0-or-later
import { formatOpenedAt, type RecentArchive } from "../lib/recents";
import { Avatar } from "../ui/Avatar";
import { AppMark, IconChevronRight, IconClose, IconFolder, IconPhone } from "../ui/icons";

/**
 * What you see before a conversation is open: a welcome, the two things you
 * can do, and the archives you opened recently.
 */
export function Home({
  recents,
  onImport,
  onOpen,
  onOpenRecent,
  onRemoveRecent,
}: {
  recents: RecentArchive[];
  onImport: () => void;
  onOpen: () => void;
  onOpenRecent: (path: string) => void;
  onRemoveRecent: (path: string) => void;
}) {
  return (
    <div className="home">
      <div className="home-inner">
        <div className="home-hero">
          <AppMark size={64} />
          <h1>Welcome to Amber</h1>
          <p>Keep your iMessage and SMS conversations safe, in a file that is yours.</p>
        </div>

        <div className="home-actions">
          <button type="button" className="btn btn-primary btn-lg" onClick={onImport}>
            <IconPhone size={18} />
            Import from iPhone
          </button>
          <button type="button" className="btn btn-lg" onClick={onOpen}>
            <IconFolder size={18} />
            Open archive…
          </button>
        </div>
        <p className="home-hint">or drop an Amber archive anywhere in this window</p>

        {recents.length > 0 && (
          <section className="home-recents" aria-label="Recent archives">
            <h2>Recent</h2>
            <div className="list-group">
              {recents.map((recent) => (
                <div className="recent-row" key={recent.path}>
                  <button type="button" className="list-row" onClick={() => onOpenRecent(recent.path)}>
                    <Avatar name={recent.name} size={32} />
                    <span className="list-row-text">
                      <span className="list-row-title">{recent.name}</span>
                      <span className="list-row-detail">
                        {recent.messageCount.toLocaleString()} messages · {formatOpenedAt(recent.openedAt)}
                      </span>
                    </span>
                    <span className="list-row-trailing">
                      <IconChevronRight size={16} />
                    </span>
                  </button>
                  <button
                    type="button"
                    className="btn btn-icon recent-remove"
                    aria-label={`Remove ${recent.name} from recent`}
                    title="Remove from recent"
                    onClick={() => onRemoveRecent(recent.path)}
                  >
                    <IconClose size={14} />
                  </button>
                </div>
              ))}
            </div>
          </section>
        )}

        <p className="home-privacy">Everything stays on this computer. Amber never sends your messages anywhere.</p>
      </div>
    </div>
  );
}
