import { useEffect, useMemo, useState } from "react";
import {
  Activity, ArrowDownLeft, ArrowUpRight, Bell, Check, CheckCheck, ChevronDown,
  ChevronRight, CircleHelp, Clipboard, Clock3, Cloud, Copy, File, FileImage,
  FileText, Fingerprint, History, Laptop, Lock, Monitor, Moon, MoreHorizontal,
  Plus, QrCode, RefreshCw, Search, Shield, ShieldCheck, Smartphone, Sun,
  Trash2, Upload, X
} from "lucide-react";
import { QRCodeSVG } from "qrcode.react";

type Page = "overview" | "history" | "devices" | "pairing" | "transfers" | "security";
type Theme = "dark" | "light" | "system";
type ClipItem = { id: number; kind: "text" | "link" | "code"; title: string; body: string; time: string; device: string; pinned?: boolean };

interface DeviceItem {
  id: string;
  name: string;
  detail: string;
  icon?: any;
  status: string;
  last: string;
  color: string;
  isRevoked?: boolean;
}

function DeviceIcon({ device, size = 18 }: { device: DeviceItem; size?: number }) {
  let IconComponent = Laptop;
  if (typeof device.icon === "function" || (typeof device.icon === "object" && device.icon !== null && "$$typeof" in (device.icon as any))) {
    IconComponent = device.icon;
  } else {
    const name = (device.name || "").toLowerCase();
    const detail = (device.detail || "").toLowerCase();
    if (name.includes("pixel") || name.includes("phone") || name.includes("android") || detail.includes("android")) {
      IconComponent = Smartphone;
    } else if (name.includes("pc") || name.includes("windows") || name.includes("desktop") || detail.includes("windows")) {
      IconComponent = Monitor;
    } else {
      IconComponent = Laptop;
    }
  }
  return <IconComponent size={size} />;
}

interface TransferItem {
  id: string;
  name: string;
  size: string;
  from: string;
  to: string;
  time: string;
  status: "Completed" | "In progress" | "Failed";
  type: "file" | "video" | "image";
}

const defaultClips: ClipItem[] = [
  { id: 1, kind: "link", title: "Project documentation", body: "https://docs.clipmesh.app/getting-started", time: "Just now", device: "Windows PC", pinned: true },
  { id: 2, kind: "text", title: "Meeting notes", body: "Sync protocol review — confirm relay acknowledgements, retry strategy, and device revocation flow.", time: "12 min ago", device: "Android phone" },
  { id: 3, kind: "code", title: "API endpoint", body: "POST /api/v1/relay/events\nAuthorization: Bearer <device-token>", time: "38 min ago", device: "Windows PC" },
  { id: 4, kind: "text", title: "Quick note", body: "Remember to review the Android background clipboard restrictions before the next build.", time: "Yesterday", device: "Linux workstation" },
  { id: 5, kind: "link", title: "Design inspiration", body: "https://linear.app/", time: "Yesterday", device: "MacBook Air" },
];

const initialDevices: DeviceItem[] = [
  { id: "d1", name: "Windows PC", detail: "This device · Windows 11", icon: Monitor, status: "Current device", last: "Active now", color: "blue" },
  { id: "d2", name: "Android phone", detail: "Pixel 8 · Android 15", icon: Smartphone, status: "Online", last: "Synced 1 min ago", color: "green" },
  { id: "d3", name: "Linux workstation", detail: "Ubuntu 24.04 · Wayland", icon: Laptop, status: "Online", last: "Synced 8 min ago", color: "green" },
  { id: "d4", name: "MacBook Air", detail: "macOS Sequoia", icon: Laptop, status: "Offline", last: "Last seen 2 hours ago", color: "gray" },
];

const initialTransfers: TransferItem[] = [
  { id: "t1", name: "design-system.fig", size: "24.8 MB", from: "Windows PC", to: "MacBook Air", time: "Today, 10:42 AM", status: "Completed", type: "file" },
  { id: "t2", name: "product-demo.mp4", size: "1.2 GB", from: "Linux workstation", to: "Windows PC", time: "Today, 09:18 AM", status: "In progress", type: "video" },
  { id: "t3", name: "Screenshots.zip", size: "86.4 MB", from: "Android phone", to: "Windows PC", time: "Yesterday, 6:32 PM", status: "Completed", type: "file" },
  { id: "t4", name: "architecture.png", size: "3.1 MB", from: "Windows PC", to: "Android phone", time: "Yesterday, 3:04 PM", status: "Failed", type: "image" },
];

const navItems: { id: Page; label: string; icon: typeof Activity }[] = [
  { id: "overview", label: "Overview", icon: Activity },
  { id: "history", label: "Clipboard history", icon: History },
  { id: "devices", label: "Devices", icon: Laptop },
  { id: "pairing", label: "Pair a device", icon: QrCode },
  { id: "transfers", label: "Transfers", icon: ArrowUpRight },
];

function App() {
  const [page, setPage] = useState<Page>("overview");
  const [theme, setTheme] = useState<Theme>(() => (localStorage.getItem("clipmesh-theme") as Theme) || "dark");
  
  const [clips, setClips] = useState<ClipItem[]>(() => {
    try {
      const saved = localStorage.getItem("clipmesh-clips");
      return saved ? JSON.parse(saved) : defaultClips;
    } catch {
      return defaultClips;
    }
  });

  const [deviceList, setDeviceList] = useState<DeviceItem[]>(() => {
    try {
      const saved = localStorage.getItem("clipmesh-devices");
      if (!saved) return initialDevices;
      const parsed: DeviceItem[] = JSON.parse(saved);
      return parsed.map(d => ({
        ...d,
        icon: typeof d.icon === "function" ? d.icon : undefined
      }));
    } catch {
      return initialDevices;
    }
  });

  const [transfers, setTransfers] = useState<TransferItem[]>(initialTransfers);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("All items");
  const [toast, setToast] = useState("");
  const [showNewTransfer, setShowNewTransfer] = useState(false);
  const [retention, setRetention] = useState(() => localStorage.getItem("clipmesh-retention") || "30 days");
  const [autoSync, setAutoSync] = useState(() => localStorage.getItem("clipmesh-autosync") !== "false");
  const [requireApproval, setRequireApproval] = useState(() => localStorage.getItem("clipmesh-require-approval") !== "false");
  const [clipboardMonitoring, setClipboardMonitoring] = useState(() => localStorage.getItem("clipmesh-monitoring") === "true");

  useEffect(() => {
    localStorage.setItem("clipmesh-clips", JSON.stringify(clips));
  }, [clips]);

  useEffect(() => {
    localStorage.setItem("clipmesh-devices", JSON.stringify(deviceList));
  }, [deviceList]);

  const notify = (message: string) => {
    setToast(message);
    window.setTimeout(() => setToast(""), 2600);
  };

  const changeTheme = (value: Theme) => {
    setTheme(value);
    localStorage.setItem("clipmesh-theme", value);
  };

  const filteredClips = useMemo(() => clips.filter((clip) => {
    const matches = `${clip.title} ${clip.body} ${clip.device}`.toLowerCase().includes(query.toLowerCase());
    const typeMatch = filter === "All items" || (filter === "Pinned" ? clip.pinned : clip.kind === filter.toLowerCase());
    return matches && typeMatch;
  }), [clips, query, filter]);

  const copyClip = async (clip: ClipItem) => {
    try {
      await navigator.clipboard.writeText(clip.body);
      notify("Copied to this device");
    } catch {
      notify("Copied: " + clip.title);
    }
  };

  const handleDeviceRevocation = (deviceId: string) => {
    setDeviceList((prev) =>
      prev.map((d) => {
        if (d.id === deviceId) {
          const nextRevoked = !d.isRevoked;
          notify(nextRevoked ? `${d.name} access revoked` : `${d.name} access restored`);
          return { ...d, isRevoked: nextRevoked };
        }
        return d;
      })
    );
  };

  const handlePairDevice = (deviceName: string, platformType: "android" | "laptop") => {
    const newDevice: DeviceItem = {
      id: `d${Date.now()}`,
      name: deviceName,
      detail: platformType === "android" ? "Android 15 · TEE Keystore" : "Linux · Secret Service",
      icon: platformType === "android" ? Smartphone : Laptop,
      status: "Online",
      last: "Active just now",
      color: "green",
      isRevoked: false,
    };
    setDeviceList((prev) => [...prev, newDevice]);
    notify(`Paired ${deviceName} successfully`);
  };

  const handleClearHistory = () => {
    setClips([]);
    notify("Clipboard history cleared");
  };

  return (
    <div className={`app-shell theme-${theme}`}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><span /></div>
          <span>clipmesh</span><span className="brand-beta">BETA</span>
        </div>
        <div className="workspace-label">WORKSPACE</div>
        <div className="workspace-switch">
          <div className="workspace-avatar">T</div>
          <div className="workspace-copy"><strong>Personal space</strong><small>End-to-end encrypted</small></div>
          <ChevronDown size={15} />
        </div>
        <div className="nav-label">YOUR SPACE</div>
        <nav className="main-nav">
          {navItems.map((item) => {
            const Icon = item.icon;
            return <button key={item.id} className={`nav-item ${page === item.id ? "active" : ""}`} onClick={() => setPage(item.id)}>
              <Icon size={17} strokeWidth={1.8} /><span>{item.label}</span>
              {item.id === "transfers" && <span className="nav-count">{transfers.filter(t => t.status === "In progress").length || transfers.length}</span>}
            </button>;
          })}
        </nav>
        <div className="nav-label second-label">PREFERENCES</div>
        <button className={`nav-item ${page === "security" ? "active" : ""}`} onClick={() => setPage("security")}><ShieldCheck size={17} /><span>Security & privacy</span></button>
        <button className="nav-item" onClick={() => notify("Documentation available in /docs")}><CircleHelp size={17} /><span>Help centre</span></button>
        <div className="sidebar-bottom">
          <div className="sync-card">
            <div className="sync-card-top"><span className="online-dot" /> All systems operational</div>
            <p>Your devices are securely connected.</p>
            <button onClick={() => setPage("devices")}>View device status <ChevronRight size={13} /></button>
          </div>
          <div className="profile-row">
            <div className="profile-avatar">T</div><div className="profile-info"><strong>Tharun</strong><small>Personal account</small></div>
            <button className="icon-button subtle" aria-label="Account menu" onClick={() => notify("ClipMesh v0.1.0 · Private build")}><MoreHorizontal size={18} /></button>
          </div>
        </div>
      </aside>

      <main className="main-area">
        <header className="topbar">
          <div className="breadcrumbs"><span>Workspace</span><ChevronRight size={14} /><strong>{navItems.find(n => n.id === page)?.label || "Security & privacy"}</strong></div>
          <div className="topbar-actions">
            <div className="connection-pill"><span className="online-dot" /> Connected</div>
            <button className="icon-button" aria-label="Notifications" onClick={() => notify("No unread alerts")}><Bell size={17} /></button>
            <div className="top-divider" />
            <div className="theme-switch" title="Theme">
              <button className={theme === "light" ? "selected" : ""} onClick={() => changeTheme("light")} aria-label="Light theme"><Sun size={15} /></button>
              <button className={theme === "system" ? "selected" : ""} onClick={() => changeTheme("system")} aria-label="System theme"><Monitor size={15} /></button>
              <button className={theme === "dark" ? "selected" : ""} onClick={() => changeTheme("dark")} aria-label="Dark theme"><Moon size={15} /></button>
            </div>
          </div>
        </header>

        <div className="content">
          {page === "overview" && <Overview clips={clips} devices={deviceList} setPage={setPage} copyClip={copyClip} />}
          {page === "history" && <HistoryPage clips={filteredClips} query={query} setQuery={setQuery} filter={filter} setFilter={setFilter} copyClip={copyClip} setClips={setClips} notify={notify} />}
          {page === "devices" && <DevicesPage devices={deviceList} onToggleRevoke={handleDeviceRevocation} setPage={setPage} />}
          {page === "pairing" && <PairingPage notify={notify} onApproveNewDevice={handlePairDevice} />}
          {page === "transfers" && <TransfersPage transfers={transfers} setTransfers={setTransfers} showNew={showNewTransfer} setShowNew={setShowNewTransfer} notify={notify} devices={deviceList} />}
          {page === "security" && (
            <SecurityPage
              retention={retention}
              setRetention={(v) => { setRetention(v); localStorage.setItem("clipmesh-retention", v); notify(`Retention updated: ${v}`); }}
              autoSync={autoSync}
              setAutoSync={(v) => { setAutoSync(v); localStorage.setItem("clipmesh-autosync", String(v)); notify(`Auto sync ${v ? "enabled" : "disabled"}`); }}
              requireApproval={requireApproval}
              setRequireApproval={(v) => { setRequireApproval(v); localStorage.setItem("clipmesh-require-approval", String(v)); notify(`Pairing approval ${v ? "required" : "auto"}`); }}
              clipboardMonitoring={clipboardMonitoring}
              setClipboardMonitoring={(v) => { setClipboardMonitoring(v); localStorage.setItem("clipmesh-monitoring", String(v)); notify(`Clipboard monitoring ${v ? "active" : "paused"}`); }}
              onClearHistory={handleClearHistory}
              notify={notify}
            />
          )}
        </div>
      </main>
      {toast && <div className="toast"><Check size={16} />{toast}</div>}
    </div>
  );
}

function PageHeading({ eyebrow, title, description, action }: { eyebrow?: string; title: string; description: string; action?: React.ReactNode }) {
  return <div className="page-heading"><div>{eyebrow && <div className="eyebrow">{eyebrow}</div>}<h1>{title}</h1><p>{description}</p></div>{action}</div>;
}
function SectionTitle({ title, action }: { title: string; action?: React.ReactNode }) {
  return <div className="section-title"><h2>{title}</h2>{action}</div>;
}
function StatCard({ icon: Icon, label, value, detail, tone }: { icon: typeof Activity; label: string; value: string; detail: string; tone: string }) {
  return <div className="stat-card"><div className={`stat-icon ${tone}`}><Icon size={18} /></div><div className="stat-label">{label}</div><div className="stat-value">{value}</div><div className="stat-detail">{detail}</div></div>;
}

function Overview({ clips, devices, setPage, copyClip }: { clips: ClipItem[]; devices: DeviceItem[]; setPage: (p: Page) => void; copyClip: (c: ClipItem) => void }) {
  const activeCount = devices.filter(d => !d.isRevoked && d.status !== "Offline").length;
  return <>
    <PageHeading eyebrow="CLIPBOARD & TRANSFERS" title="Good day, Tharun" description="Everything you copy, ready on every device." action={<button className="button primary" onClick={() => setPage("pairing")}><Plus size={16} /> Pair a device</button>} />
    <div className="status-banner"><div className="status-orbit"><Cloud size={20} /></div><div className="status-copy"><strong>Your mesh is in sync</strong><span>{activeCount} devices active in your private end-to-end encrypted network</span></div><div className="status-right"><span className="online-dot" /> Live sync active</div></div>
    <div className="stats-grid">
      <StatCard icon={Laptop} label="Connected devices" value={String(devices.filter(d => !d.isRevoked).length)} detail={`${activeCount} online right now`} tone="blue" />
      <StatCard icon={Clipboard} label="Clipboard items" value={String(clips.length)} detail="Synced across devices" tone="violet" />
      <StatCard icon={ArrowUpRight} label="Transfers this week" value="18" detail="End-to-end encrypted" tone="amber" />
      <StatCard icon={ShieldCheck} label="Security status" value="Protected" detail="Zero-knowledge relay" tone="green" />
    </div>
    <div className="overview-grid">
      <section className="panel recent-panel">
        <SectionTitle title="Recent clipboard" action={<button className="text-link" onClick={() => setPage("history")}>View history <ChevronRight size={14} /></button>} />
        <div className="clip-list">
          {clips.slice(0, 4).map(clip => <ClipRow key={clip.id} clip={clip} onCopy={() => copyClip(clip)} compact />)}
        </div>
      </section>
      <section className="panel devices-panel">
        <SectionTitle title="Your devices" action={<button className="text-link" onClick={() => setPage("devices")}>Manage <ChevronRight size={14} /></button>} />
        {devices.slice(0, 3).map((device) => {
          const isOffline = device.status === "Offline" || device.isRevoked;
          return <div className="mini-device" key={device.id}>
            <div className="device-icon"><DeviceIcon device={device} size={18} /></div>
            <div className="device-meta"><strong>{device.name}</strong><small>{device.isRevoked ? "Access revoked" : device.last}</small></div>
            <span className={`status-dot ${isOffline ? "offline" : ""}`} />
          </div>;
        })}
        <button className="add-device-row" onClick={() => setPage("pairing")}><Plus size={15} /> Add another device</button>
      </section>
    </div>
    <section className="panel activity-panel"><SectionTitle title="Recent activity" action={<button className="text-link" onClick={() => setPage("transfers")}>All activity <ChevronRight size={14} /></button>} />
      <div className="activity-row"><div className="activity-icon blue"><ArrowDownLeft size={16} /></div><div><strong>Clipboard received</strong><small>Meeting notes · Android phone</small></div><time>12 min ago</time><span className="activity-state">Synced</span></div>
      <div className="activity-row"><div className="activity-icon violet"><Upload size={16} /></div><div><strong>File transfer completed</strong><small>design-system.fig · 24.8 MB</small></div><time>1 hour ago</time><span className="activity-state">Complete</span></div>
      <div className="activity-row"><div className="activity-icon green"><Shield size={16} /></div><div><strong>Device verified</strong><small>Linux workstation · Trusted device</small></div><time>Yesterday</time><span className="activity-state">Secure</span></div>
    </section>
    <div className="privacy-footnote"><Lock size={14} /> Your clipboard is encrypted on your devices. ClipMesh servers only relay encrypted data.</div>
  </>;
}

function ClipRow({ clip, onCopy, compact = false, onPin, onDelete }: { clip: ClipItem; onCopy: () => void; compact?: boolean; onPin?: () => void; onDelete?: () => void }) {
  const Icon = clip.kind === "link" ? FileImage : clip.kind === "code" ? FileText : Clipboard;
  return <div className="clip-row"><div className={`clip-type ${clip.kind}`}><Icon size={17} /></div><div className="clip-main"><div className="clip-title">{clip.title}{clip.pinned && <span className="pin-tag">PINNED</span>}</div><p className="clip-body">{clip.body}</p><div className="clip-meta"><span>{clip.device}</span><span className="meta-dot">·</span><span>{clip.time}</span></div></div><div className="clip-actions"><button className="icon-button" title="Copy" onClick={onCopy}><Copy size={15} /></button>{!compact && <><button className="icon-button" title={clip.pinned ? "Unpin" : "Pin"} onClick={onPin}><ChevronDown size={15} /></button><button className="icon-button danger-hover" title="Delete" onClick={onDelete}><Trash2 size={15} /></button></>}</div></div>;
}

function HistoryPage({ clips, query, setQuery, filter, setFilter, copyClip, setClips, notify }: { clips: ClipItem[]; query: string; setQuery: (s: string) => void; filter: string; setFilter: (s: string) => void; copyClip: (c: ClipItem) => void; setClips: React.Dispatch<React.SetStateAction<ClipItem[]>>; notify: (s: string) => void }) {
  const handleClearUnpinned = () => {
    setClips(c => c.filter(x => x.pinned));
    notify("Cleared unpinned clipboard items");
  };

  return <>
    <PageHeading eyebrow="YOUR DATA" title="Clipboard history" description="A private timeline of what you've copied across your devices." action={<button className="button secondary" onClick={handleClearUnpinned}><Trash2 size={15} /> Clear unpinned</button>} />
    <div className="history-toolbar"><div className="search-box"><Search size={16} /><input value={query} onChange={e => setQuery(e.target.value)} placeholder="Search your clipboard..." /><kbd>⌘ K</kbd></div><div className="filter-pills">{["All items", "Text", "Link", "Code", "Pinned"].map(f => <button key={f} className={filter === f ? "selected" : ""} onClick={() => setFilter(f)}>{f}</button>)}</div></div>
    <div className="history-summary"><span><strong>{clips.length}</strong> items found</span><span><Lock size={13} /> Stored locally, encrypted</span></div>
    <section className="panel history-list">{clips.length ? clips.map(clip => <ClipRow key={clip.id} clip={clip} onCopy={() => copyClip(clip)} onPin={() => setClips(cs => cs.map(c => c.id === clip.id ? { ...c, pinned: !c.pinned } : c))} onDelete={() => { setClips(cs => cs.filter(c => c.id !== clip.id)); notify("Item deleted"); }} />) : <div className="empty-state"><Clipboard size={28} /><strong>No clipboard items found</strong><span>Try another search or filter.</span></div>}</section>
    <p className="privacy-footnote"><Clock3 size={14} /> History automatically expires based on your retention settings.</p>
  </>;
}

function DevicesPage({ devices, onToggleRevoke, setPage }: { devices: DeviceItem[]; onToggleRevoke: (id: string) => void; setPage: (p: Page) => void }) {
  const activeCount = devices.filter(d => !d.isRevoked && d.status !== "Offline").length;

  return <>
    <PageHeading eyebrow="YOUR NETWORK" title="Trusted devices" description="Manage the devices that can access your ClipMesh network." action={<button className="button primary" onClick={() => setPage("pairing")}><Plus size={16} /> Add device</button>} />
    <div className="device-summary"><div><span className="online-dot" /> <strong>{activeCount} devices online</strong><small> · {devices.length} devices registered</small></div><span className="encrypted-label"><ShieldCheck size={14} /> All connections encrypted</span></div>
    <div className="device-grid">{devices.map(device => {
      const isRevoked = !!device.isRevoked;
      return <div className={`panel device-card ${isRevoked ? "revoked" : ""}`} key={device.id}>
        <div className="device-card-top"><div className="device-icon large"><DeviceIcon device={device} size={21} /></div><button className="icon-button" aria-label="Device options"><MoreHorizontal size={18} /></button></div>
        <h3>{device.name}</h3><p>{device.detail}</p>
        <div className="device-status-line"><span className={`status-dot ${device.status === "Offline" || isRevoked ? "offline" : ""}`} />{isRevoked ? "Access revoked" : device.status}<span className="device-last">{isRevoked ? "Blocked" : device.last}</span></div>
        <div className="device-card-footer">{device.status === "Current device" ? <span className="current-device">THIS DEVICE</span> : <button className={`button small ${isRevoked ? "secondary" : "danger"}`} onClick={() => onToggleRevoke(device.id)}>{isRevoked ? "Restore access" : "Revoke access"}</button>}</div>
      </div>;
    })}</div>
    <div className="notice-card"><Shield size={18} /><div><strong>Lost a device?</strong><p>Revoke its access immediately. A revoked device can no longer receive new clipboard events or transfers.</p></div><button className="text-link" onClick={() => setPage("security")}>Learn more <ChevronRight size={14} /></button></div>
  </>;
}

function PairingPage({ notify, onApproveNewDevice }: { notify: (s: string) => void; onApproveNewDevice: (name: string, type: "android" | "laptop") => void }) {
  const [code, setCode] = useState("CM-7K4P-92QX");
  const [timeLeft, setTimeLeft] = useState(118);
  const [approved, setApproved] = useState(false);

  useEffect(() => {
    const timer = setInterval(() => {
      setTimeLeft(t => (t > 0 ? t - 1 : 0));
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const regenerate = () => {
    const part1 = Math.random().toString(36).slice(2, 6).toUpperCase();
    const part2 = Math.random().toString(36).slice(2, 6).toUpperCase();
    setCode(`CM-${part1}-${part2}`);
    setTimeLeft(120);
    setApproved(false);
    notify("A new cryptographic pairing code was generated");
  };

  const minutes = String(Math.floor(timeLeft / 60)).padStart(2, "0");
  const seconds = String(timeLeft % 60).padStart(2, "0");

  const handleSimulateApproval = () => {
    if (!approved) {
      setApproved(true);
      onApproveNewDevice("Pixel 9 Pro", "android");
    } else {
      setApproved(false);
    }
  };

  return <>
    <PageHeading eyebrow="EXPAND YOUR MESH" title="Pair a new device" description="Connect a device to your private mesh in a few seconds." />
    <div className="pair-layout">
      <section className="panel pair-card"><div className="step-label"><span>01</span> SCAN TO PAIR</div><h2>Open ClipMesh on your new device</h2><p>Choose <strong>Pair existing device</strong> and scan this one-time QR code.</p>
        <div className="qr-frame"><QRCodeSVG value={`clipmesh://pair?v=1&code=${code}&ep=${encodeURIComponent((typeof window !== "undefined" && (window.location.hostname === "localhost" || window.location.hostname === "127.0.0.1") ? (window.location.protocol + '//192.168.1.64:8085') : (window.location.protocol + '//' + window.location.hostname + ':8085')))}`} size={190} bgColor="#ffffff" fgColor="#111827" level="H" includeMargin /></div>
        <div className="pair-code-label">OR ENTER THIS CODE</div><div className="pair-code">{code}<button className="icon-button" onClick={() => { navigator.clipboard?.writeText(code); notify("Pairing code copied"); }}><Copy size={15} /></button></div>
        <div className="pair-expiry"><Clock3 size={14} /> Expires in {minutes}:{seconds} <span>·</span> Relay: <code>{(typeof window !== "undefined" && (window.location.hostname === "localhost" || window.location.hostname === "127.0.0.1")) ? "192.168.1.64:8085" : window.location.hostname + ":8085"}</code></div>
        <button className="button secondary full" onClick={regenerate}><RefreshCw size={15} /> Generate new code</button>
      </section>
      <section className="pair-side">
        <div className="panel pairing-steps"><div className="step-label">HOW IT WORKS</div><div className="pair-step"><div className="step-number">1</div><div><strong>Scan the QR code</strong><p>Use the ClipMesh app on your new device to scan this code.</p></div></div><div className="pair-step"><div className="step-number">2</div><div><strong>Verify the device</strong><p>Confirm the device name and verify the short authentication phrase.</p></div></div><div className="pair-step"><div className="step-number">3</div><div><strong>You're connected</strong><p>Your devices establish an encrypted connection and start syncing.</p></div></div></div>
        <div className="security-callout"><ShieldCheck size={19} /><div><strong>Secure by design</strong><p>Pairing codes are short-lived and single-use. Your private keys never leave your devices.</p></div></div>
        <div className={`approval-card ${approved ? "approved" : ""}`}><div className="approval-icon">{approved ? <CheckCheck size={18} /> : <Clock3 size={18} />}</div><div><strong>{approved ? "Device verified & connected" : "Waiting for device scan"}</strong><p>{approved ? "New device added to your trusted mesh." : "When a device scans the code, its request will appear here."}</p></div><button className="button small secondary" onClick={handleSimulateApproval}>{approved ? "Reset" : "Pair new device"}</button></div>
      </section>
    </div>
  </>;
}

function TransfersPage({
  transfers,
  setTransfers,
  showNew,
  setShowNew,
  notify,
  devices,
}: {
  transfers: TransferItem[];
  setTransfers: React.Dispatch<React.SetStateAction<TransferItem[]>>;
  showNew: boolean;
  setShowNew: (b: boolean) => void;
  notify: (s: string) => void;
  devices: DeviceItem[];
}) {
  const [tab, setTab] = useState("All transfers");
  const [fileName, setFileName] = useState("project-assets.zip");
  const [targetDevice, setTargetDevice] = useState(devices[1]?.name || "Android phone");

  const startNewTransfer = () => {
    const newItem: TransferItem = {
      id: `t${Date.now()}`,
      name: fileName,
      size: "14.2 MB",
      from: "Windows PC",
      to: targetDevice,
      time: "Just now",
      status: "In progress",
      type: "file",
    };
    setTransfers(prev => [newItem, ...prev]);
    setShowNew(false);
    notify(`Transfer initiated: ${fileName} -> ${targetDevice}`);

    // Simulate transfer completing after 2 seconds
    setTimeout(() => {
      setTransfers(prev =>
        prev.map(t => (t.id === newItem.id ? { ...t, status: "Completed" } : t))
      );
      notify(`Transfer completed: ${fileName}`);
    }, 2500);
  };

  return <>
    <PageHeading eyebrow="MOVE WITHOUT LIMITS" title="File transfers" description="Send files between your trusted devices. Your files stay encrypted in transit." action={<button className="button primary" onClick={() => setShowNew(true)}><Upload size={16} /> New transfer</button>} />
    <div className="transfer-metrics">
      <div className="panel metric-inline"><div className="metric-icon violet"><ArrowUpRight size={18} /></div><div><small>Sent this week</small><strong>4.8 GB</strong></div></div>
      <div className="panel metric-inline"><div className="metric-icon blue"><ArrowDownLeft size={18} /></div><div><small>Received this week</small><strong>1.6 GB</strong></div></div>
      <div className="panel metric-inline"><div className="metric-icon green"><CheckCheck size={18} /></div><div><small>Completed transfers</small><strong>{transfers.filter(t => t.status === "Completed").length}</strong></div></div>
    </div>
    <div className="transfer-toolbar"><div className="filter-pills">{["All transfers", "Sending", "Received", "Failed"].map(t => <button key={t} className={tab === t ? "selected" : ""} onClick={() => setTab(t)}>{t}</button>)}</div><span className="history-summary">Last 30 days <ChevronDown size={14} /></span></div>
    <section className="panel transfer-table"><div className="table-head"><span>FILE</span><span>ROUTE</span><span>DATE</span><span>STATUS</span><span /></div>{transfers.filter(r => tab === "All transfers" || (tab === "Failed" ? r.status === "Failed" : tab === "Sending" ? r.from === "Windows PC" : r.to === "Windows PC")).map((r) => <div className="table-row" key={r.id}><div className="file-cell"><div className="file-icon"><File size={17} /></div><div><strong>{r.name}</strong><small>{r.size}</small></div></div><div className="route-cell"><span>{r.from}</span><ArrowRightIcon /><span>{r.to}</span></div><span className="date-cell">{r.time}</span><span className={`transfer-status ${r.status.toLowerCase().replace(" ", "-")}`}>{r.status}</span><button className="icon-button" onClick={() => notify(`${r.name} SHA-256 verified`)}><MoreHorizontal size={17} /></button></div>)}</section>
    {showNew && <div className="modal-backdrop" onClick={() => setShowNew(false)}><div className="modal panel" onClick={e => e.stopPropagation()}><div className="modal-head"><div><div className="eyebrow">SEND A FILE</div><h2>New transfer</h2></div><button className="icon-button" onClick={() => setShowNew(false)}><X size={18} /></button></div><div className="drop-zone"><Upload size={25} /><strong>Drop files here</strong><span>or select a file from this device</span><input type="text" className="search-box" style={{ background: "transparent", border: "1px solid var(--border)", padding: "8px 12px", borderRadius: "8px", marginTop: "10px", width: "80%", color: "var(--text)" }} value={fileName} onChange={e => setFileName(e.target.value)} placeholder="Filename..." /></div><label className="field-label">SEND TO</label><select className="select-field" value={targetDevice} onChange={e => setTargetDevice(e.target.value)}>{devices.filter(d => !d.isRevoked && d.status !== "Current device").map(d => <option key={d.id} value={d.name}>{d.name}</option>)}</select><div className="modal-actions"><button className="button secondary" onClick={() => setShowNew(false)}>Cancel</button><button className="button primary" onClick={startNewTransfer}>Send encrypted file <ChevronRight size={15} /></button></div></div></div>}
    <div className="privacy-footnote"><Lock size={14} /> Transfers are end-to-end encrypted. Cloud relay is used when direct peer-to-peer transfer isn't available.</div>
  </>;
}

function ArrowRightIcon() { return <ChevronRight size={13} className="route-arrow" />; }

function SecurityPage({
  retention,
  setRetention,
  autoSync,
  setAutoSync,
  requireApproval,
  setRequireApproval,
  clipboardMonitoring,
  setClipboardMonitoring,
  onClearHistory,
  notify,
}: {
  retention: string;
  setRetention: (s: string) => void;
  autoSync: boolean;
  setAutoSync: (b: boolean) => void;
  requireApproval: boolean;
  setRequireApproval: (b: boolean) => void;
  clipboardMonitoring: boolean;
  setClipboardMonitoring: (b: boolean) => void;
  onClearHistory: () => void;
  notify: (s: string) => void;
}) {
  return <>
    <PageHeading eyebrow="YOUR PRIVACY, YOUR RULES" title="Security & privacy" description="Control how your data is protected, synced, and stored." />
    <div className="security-layout"><div className="security-main">
      <section className="panel settings-panel"><div className="settings-heading"><div className="settings-icon green"><ShieldCheck size={18} /></div><div><h2>Encryption</h2><p>Your data is encrypted before it leaves your device.</p></div><span className="verified-pill"><Check size={13} /> Active</span></div>
        <div className="setting-row"><div><strong>End-to-end encryption</strong><p>Only your trusted devices can decrypt synced content.</p></div><span className="locked-value"><Lock size={13} /> Always on</span></div>
        <div className="setting-row"><div><strong>Key management</strong><p>Encryption keys are generated and stored on your devices.</p></div><button className="text-link" onClick={() => notify("Keys stored securely in OS Keystore")}>View details <ChevronRight size={14} /></button></div>
      </section>
      <section className="panel settings-panel"><div className="settings-heading"><div className="settings-icon blue"><RefreshCw size={18} /></div><div><h2>Sync preferences</h2><p>Choose when and how your devices stay in sync.</p></div></div>
        <ToggleRow title="Automatic sync" description="Sync clipboard items when your trusted devices are online." checked={autoSync} onChange={setAutoSync} />
        <ToggleRow title="Ask before receiving" description="Require confirmation before accepting a new device's pairing request." checked={requireApproval} onChange={setRequireApproval} />
        <ToggleRow title="Clipboard monitoring" description="Allow the desktop app to watch clipboard changes while running. Disabled by default." checked={clipboardMonitoring} onChange={setClipboardMonitoring} />
      </section>
      <section className="panel settings-panel"><div className="settings-heading"><div className="settings-icon violet"><History size={18} /></div><div><h2>Clipboard history</h2><p>Set how long your local encrypted history is retained.</p></div></div>
        <div className="setting-row"><div><strong>Retention period</strong><p>Older items are automatically removed from this device.</p></div><select className="select-field compact-select" value={retention} onChange={e => setRetention(e.target.value)}><option>24 hours</option><option>7 days</option><option>30 days</option><option>90 days</option><option>Forever</option></select></div>
        <div className="setting-row"><div><strong>Clear clipboard history</strong><p>Remove all locally stored clipboard items from this device.</p></div><button className="button danger small" onClick={onClearHistory}>Clear history</button></div>
      </section>
    </div><aside className="security-aside">
      <div className="panel security-score"><div className="score-ring"><ShieldCheck size={26} /></div><div className="eyebrow">SECURITY OVERVIEW</div><h2>Looking good.</h2><p>Your ClipMesh setup follows the recommended security baseline.</p><div className="score-check"><Check size={14} /> End-to-end encryption enabled</div><div className="score-check"><Check size={14} /> Trusted devices only</div><div className="score-check"><Check size={14} /> Pairing approval enabled</div></div>
      <div className="panel key-card"><Fingerprint size={20} /><strong>Device identity</strong><p>Each device has its own cryptographic identity. Revoking a device blocks future access.</p><button className="text-link" onClick={() => notify("Ed25519 + X25519 identity keypairs verified")}>Learn about device keys <ChevronRight size={14} /></button></div>
    </aside></div>
    <div className="privacy-footnote"><Lock size={14} /> ClipMesh cannot read your clipboard content. Cryptographic operations occur locally on your devices.</div>
  </>;
}

function ToggleRow({ title, description, checked, onChange }: { title: string; description: string; checked: boolean; onChange: (b: boolean) => void }) {
  return <div className="setting-row"><div><strong>{title}</strong><p>{description}</p></div><button className={`toggle ${checked ? "on" : ""}`} role="switch" aria-checked={checked} onClick={() => onChange(!checked)}><span /></button></div>;
}

export default App;
