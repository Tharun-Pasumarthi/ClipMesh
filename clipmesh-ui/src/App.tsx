import { useMemo, useState } from "react";
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

const initialClips: ClipItem[] = [
  { id: 1, kind: "link", title: "Project documentation", body: "https://docs.clipmesh.app/getting-started", time: "Just now", device: "Windows PC", pinned: true },
  { id: 2, kind: "text", title: "Meeting notes", body: "Sync protocol review — confirm relay acknowledgements, retry strategy, and device revocation flow.", time: "12 min ago", device: "Android phone" },
  { id: 3, kind: "code", title: "API endpoint", body: "POST /api/v1/relay/events\\nAuthorization: Bearer <device-token>", time: "38 min ago", device: "Windows PC" },
  { id: 4, kind: "text", title: "Quick note", body: "Remember to review the Android background clipboard restrictions before the next build.", time: "Yesterday", device: "Linux workstation" },
  { id: 5, kind: "link", title: "Design inspiration", body: "https://linear.app/", time: "Yesterday", device: "MacBook Air" },
];

const devices = [
  { id: "d1", name: "Windows PC", detail: "This device · Windows 11", icon: Monitor, status: "Current device", last: "Active now", color: "blue" },
  { id: "d2", name: "Android phone", detail: "Pixel 8 · Android 15", icon: Smartphone, status: "Online", last: "Synced 1 min ago", color: "green" },
  { id: "d3", name: "Linux workstation", detail: "Ubuntu 24.04 · Wayland", icon: Laptop, status: "Online", last: "Synced 8 min ago", color: "green" },
  { id: "d4", name: "MacBook Air", detail: "macOS Sequoia", icon: Laptop, status: "Offline", last: "Last seen 2 hours ago", color: "gray" },
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
  const [clips, setClips] = useState(initialClips);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("All items");
  const [revoked, setRevoked] = useState<string[]>([]);
  const [toast, setToast] = useState("");
  const [showNewTransfer, setShowNewTransfer] = useState(false);
  const [retention, setRetention] = useState("30 days");
  const [autoSync, setAutoSync] = useState(true);
  const [requireApproval, setRequireApproval] = useState(true);
  const [clipboardMonitoring, setClipboardMonitoring] = useState(false);

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
    try { await navigator.clipboard.writeText(clip.body); notify("Copied to this device"); }
    catch { notify("Clipboard permission is unavailable in this preview"); }
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
          <div className="workspace-copy"><strong>Personal space</strong><small>Free plan</small></div>
          <ChevronDown size={15} />
        </div>
        <div className="nav-label">YOUR SPACE</div>
        <nav className="main-nav">
          {navItems.map((item) => {
            const Icon = item.icon;
            return <button key={item.id} className={`nav-item ${page === item.id ? "active" : ""}`} onClick={() => setPage(item.id)}>
              <Icon size={17} strokeWidth={1.8} /><span>{item.label}</span>
              {item.id === "transfers" && <span className="nav-count">2</span>}
            </button>;
          })}
        </nav>
        <div className="nav-label second-label">PREFERENCES</div>
        <button className={`nav-item ${page === "security" ? "active" : ""}`} onClick={() => setPage("security")}><ShieldCheck size={17} /><span>Security & privacy</span></button>
        <button className="nav-item" onClick={() => notify("Help centre coming soon")}><CircleHelp size={17} /><span>Help centre</span></button>
        <div className="sidebar-bottom">
          <div className="sync-card">
            <div className="sync-card-top"><span className="online-dot" /> All systems operational</div>
            <p>Your devices are securely connected.</p>
            <button onClick={() => setPage("devices")}>View device status <ChevronRight size={13} /></button>
          </div>
          <div className="profile-row">
            <div className="profile-avatar">T</div><div className="profile-info"><strong>Tharun</strong><small>Personal account</small></div>
            <button className="icon-button subtle" aria-label="Account menu" onClick={() => notify("Account menu coming soon")}><MoreHorizontal size={18} /></button>
          </div>
        </div>
      </aside>

      <main className="main-area">
        <header className="topbar">
          <div className="breadcrumbs"><span>Workspace</span><ChevronRight size={14} /><strong>{navItems.find(n => n.id === page)?.label || "Security & privacy"}</strong></div>
          <div className="topbar-actions">
            <div className="connection-pill"><span className="online-dot" /> Connected</div>
            <button className="icon-button" aria-label="Notifications" onClick={() => notify("You're all caught up")}><Bell size={17} /></button>
            <div className="top-divider" />
            <div className="theme-switch" title="Theme">
              <button className={theme === "light" ? "selected" : ""} onClick={() => changeTheme("light")} aria-label="Light theme"><Sun size={15} /></button>
              <button className={theme === "system" ? "selected" : ""} onClick={() => changeTheme("system")} aria-label="System theme"><Monitor size={15} /></button>
              <button className={theme === "dark" ? "selected" : ""} onClick={() => changeTheme("dark")} aria-label="Dark theme"><Moon size={15} /></button>
            </div>
          </div>
        </header>

        <div className="content">
          {page === "overview" && <Overview clips={clips} setPage={setPage} copyClip={copyClip} />}
          {page === "history" && <HistoryPage clips={filteredClips} query={query} setQuery={setQuery} filter={filter} setFilter={setFilter} copyClip={copyClip} setClips={setClips} />}
          {page === "devices" && <DevicesPage revoked={revoked} setRevoked={setRevoked} notify={notify} setPage={setPage} />}
          {page === "pairing" && <PairingPage notify={notify} />}
          {page === "transfers" && <TransfersPage showNew={showNewTransfer} setShowNew={setShowNewTransfer} notify={notify} />}
          {page === "security" && <SecurityPage retention={retention} setRetention={setRetention} autoSync={autoSync} setAutoSync={setAutoSync} requireApproval={requireApproval} setRequireApproval={setRequireApproval} clipboardMonitoring={clipboardMonitoring} setClipboardMonitoring={setClipboardMonitoring} notify={notify} />}
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
function Overview({ clips, setPage, copyClip }: { clips: ClipItem[]; setPage: (p: Page) => void; copyClip: (c: ClipItem) => void }) {
  return <>
    <PageHeading eyebrow="TUESDAY, NOVEMBER 3" title="Good morning, Tharun" description="Everything you copy, ready on every device." action={<button className="button primary" onClick={() => setPage("pairing")}><Plus size={16} /> Pair a device</button>} />
    <div className="status-banner"><div className="status-orbit"><Cloud size={20} /></div><div className="status-copy"><strong>Your mesh is in sync</strong><span>4 devices connected to your private network</span></div><div className="status-right"><span className="online-dot" /> Live sync active</div></div>
    <div className="stats-grid">
      <StatCard icon={Laptop} label="Connected devices" value="4" detail="3 online right now" tone="blue" />
      <StatCard icon={Clipboard} label="Clipboard items" value="128" detail="+12 synced today" tone="violet" />
      <StatCard icon={ArrowUpRight} label="Transfers this week" value="18" detail="6.4 GB shared" tone="amber" />
      <StatCard icon={ShieldCheck} label="Security status" value="Protected" detail="End-to-end encrypted" tone="green" />
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
          const Icon = device.icon;
          return <div className="mini-device" key={device.id}><div className="device-icon"><Icon size={18} /></div><div className="device-meta"><strong>{device.name}</strong><small>{device.last}</small></div><span className={`status-dot ${device.status === "Offline" ? "offline" : ""}`} /></div>;
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
function HistoryPage({ clips, query, setQuery, filter, setFilter, copyClip, setClips }: { clips: ClipItem[]; query: string; setQuery: (s: string) => void; filter: string; setFilter: (s: string) => void; copyClip: (c: ClipItem) => void; setClips: React.Dispatch<React.SetStateAction<ClipItem[]>> }) {
  return <>
    <PageHeading eyebrow="YOUR DATA" title="Clipboard history" description="A private timeline of what you've copied across your devices." action={<button className="button secondary" onClick={() => setClips(c => c.filter(x => x.pinned))}><Trash2 size={15} /> Clear unpinned</button>} />
    <div className="history-toolbar"><div className="search-box"><Search size={16} /><input value={query} onChange={e => setQuery(e.target.value)} placeholder="Search your clipboard..." /><kbd>⌘ K</kbd></div><div className="filter-pills">{["All items", "Text", "Link", "Code", "Pinned"].map(f => <button key={f} className={filter === f ? "selected" : ""} onClick={() => setFilter(f)}>{f}</button>)}</div></div>
    <div className="history-summary"><span><strong>{clips.length}</strong> items found</span><span><Lock size={13} /> Stored locally, encrypted</span></div>
    <section className="panel history-list">{clips.length ? clips.map(clip => <ClipRow key={clip.id} clip={clip} onCopy={() => copyClip(clip)} onPin={() => setClips(cs => cs.map(c => c.id === clip.id ? { ...c, pinned: !c.pinned } : c))} onDelete={() => setClips(cs => cs.filter(c => c.id !== clip.id))} />) : <div className="empty-state"><Clipboard size={28} /><strong>No clipboard items found</strong><span>Try another search or filter.</span></div>}</section>
    <p className="privacy-footnote"><Clock3 size={14} /> History automatically expires based on your retention settings.</p>
  </>;
}

function DevicesPage({ revoked, setRevoked, notify, setPage }: { revoked: string[]; setRevoked: React.Dispatch<React.SetStateAction<string[]>>; notify: (s: string) => void; setPage: (p: Page) => void }) {
  return <>
    <PageHeading eyebrow="YOUR NETWORK" title="Trusted devices" description="Manage the devices that can access your ClipMesh network." action={<button className="button primary" onClick={() => setPage("pairing")}><Plus size={16} /> Add device</button>} />
    <div className="device-summary"><div><span className="online-dot" /> <strong>{devices.filter(d => !revoked.includes(d.id) && d.status !== "Offline").length} devices online</strong><small> · 4 devices registered</small></div><span className="encrypted-label"><ShieldCheck size={14} /> All connections encrypted</span></div>
    <div className="device-grid">{devices.map(device => {
      const Icon = device.icon; const isRevoked = revoked.includes(device.id);
      return <div className={`panel device-card ${isRevoked ? "revoked" : ""}`} key={device.id}>
        <div className="device-card-top"><div className="device-icon large"><Icon size={21} /></div><button className="icon-button" onClick={() => notify(`${device.name} options`)}><MoreHorizontal size={18} /></button></div>
        <h3>{device.name}</h3><p>{device.detail}</p>
        <div className="device-status-line"><span className={`status-dot ${device.status === "Offline" || isRevoked ? "offline" : ""}`} />{isRevoked ? "Access revoked" : device.status}<span className="device-last">{device.last}</span></div>
        <div className="device-card-footer">{device.status === "Current device" ? <span className="current-device">THIS DEVICE</span> : <button className={`button small ${isRevoked ? "secondary" : "danger"}`} onClick={() => { setRevoked(r => isRevoked ? r.filter(x => x !== device.id) : [...r, device.id]); notify(isRevoked ? "Device access restored in this demo" : "Device access revoked in this demo"); }}>{isRevoked ? "Restore access" : "Revoke access"}</button>}</div>
      </div>;
    })}</div>
    <div className="notice-card"><Shield size={18} /><div><strong>Lost a device?</strong><p>Revoke its access immediately. A revoked device can no longer receive new clipboard events or transfers.</p></div><button className="text-link" onClick={() => notify("Recovery guide coming soon")}>Learn more <ChevronRight size={14} /></button></div>
  </>;
}

function PairingPage({ notify }: { notify: (s: string) => void }) {
  const [code, setCode] = useState("CM-7K4P-92QX");
  const [approved, setApproved] = useState(false);
  const regenerate = () => { setCode(`CM-${Math.random().toString(36).slice(2, 6).toUpperCase()}-${Math.random().toString(36).slice(2, 6).toUpperCase()}`); setApproved(false); notify("A new pairing code was generated"); };
  return <>
    <PageHeading eyebrow="EXPAND YOUR MESH" title="Pair a new device" description="Connect a device to your private mesh in a few seconds." />
    <div className="pair-layout">
      <section className="panel pair-card"><div className="step-label"><span>01</span> SCAN TO PAIR</div><h2>Open ClipMesh on your new device</h2><p>Choose <strong>Pair existing device</strong> and scan this one-time QR code.</p>
        <div className="qr-frame"><QRCodeSVG value={`clipmesh://pair?code=${code}&v=1`} size={190} bgColor="#ffffff" fgColor="#111827" level="H" includeMargin /></div>
        <div className="pair-code-label">OR ENTER THIS CODE</div><div className="pair-code">{code}<button className="icon-button" onClick={() => { navigator.clipboard?.writeText(code); notify("Pairing code copied"); }}><Copy size={15} /></button></div>
        <div className="pair-expiry"><Clock3 size={14} /> Expires in 01:58 <span>·</span> Single use only</div>
        <button className="button secondary full" onClick={regenerate}><RefreshCw size={15} /> Generate new code</button>
      </section>
      <section className="pair-side">
        <div className="panel pairing-steps"><div className="step-label">HOW IT WORKS</div><div className="pair-step"><div className="step-number">1</div><div><strong>Scan the QR code</strong><p>Use the ClipMesh app on your new device to scan this code.</p></div></div><div className="pair-step"><div className="step-number">2</div><div><strong>Verify the device</strong><p>Confirm the device name and verify the short authentication phrase.</p></div></div><div className="pair-step"><div className="step-number">3</div><div><strong>You're connected</strong><p>Your devices establish an encrypted connection and start syncing.</p></div></div></div>
        <div className="security-callout"><ShieldCheck size={19} /><div><strong>Secure by design</strong><p>Pairing codes are short-lived and single-use. Your private keys never leave your devices.</p></div></div>
        <div className={`approval-card ${approved ? "approved" : ""}`}><div className="approval-icon">{approved ? <CheckCheck size={18} /> : <Clock3 size={18} />}</div><div><strong>{approved ? "Demo approval recorded" : "Waiting for device approval"}</strong><p>{approved ? "This is a UI preview; no real device was paired." : "When a device scans the code, its request will appear here."}</p></div><button className="button small secondary" onClick={() => setApproved(!approved)}>{approved ? "Reset" : "Preview approval"}</button></div>
      </section>
    </div>
  </>;
}

function TransfersPage({ showNew, setShowNew, notify }: { showNew: boolean; setShowNew: (b: boolean) => void; notify: (s: string) => void }) {
  const [tab, setTab] = useState("All transfers");
  const transferRows = [
    { name: "design-system.fig", size: "24.8 MB", from: "Windows PC", to: "MacBook Air", time: "Today, 10:42 AM", status: "Completed", type: "file" },
    { name: "product-demo.mp4", size: "1.2 GB", from: "Linux workstation", to: "Windows PC", time: "Today, 09:18 AM", status: "In progress", type: "video" },
    { name: "Screenshots.zip", size: "86.4 MB", from: "Android phone", to: "Windows PC", time: "Yesterday, 6:32 PM", status: "Completed", type: "file" },
    { name: "architecture.png", size: "3.1 MB", from: "Windows PC", to: "Android phone", time: "Yesterday, 3:04 PM", status: "Failed", type: "image" },
  ];
  return <>
    <PageHeading eyebrow="MOVE WITHOUT LIMITS" title="File transfers" description="Send files between your trusted devices. Your files stay encrypted in transit." action={<button className="button primary" onClick={() => setShowNew(true)}><Upload size={16} /> New transfer</button>} />
    <div className="transfer-metrics"><div className="panel metric-inline"><div className="metric-icon violet"><ArrowUpRight size={18} /></div><div><small>Sent this week</small><strong>4.8 GB</strong></div></div><div className="panel metric-inline"><div className="metric-icon blue"><ArrowDownLeft size={18} /></div><div><small>Received this week</small><strong>1.6 GB</strong></div></div><div className="panel metric-inline"><div className="metric-icon green"><CheckCheck size={18} /></div><div><small>Completed transfers</small><strong>16</strong></div></div></div>
    <div className="transfer-toolbar"><div className="filter-pills">{["All transfers", "Sending", "Received", "Failed"].map(t => <button key={t} className={tab === t ? "selected" : ""} onClick={() => setTab(t)}>{t}</button>)}</div><span className="history-summary">Last 30 days <ChevronDown size={14} /></span></div>
    <section className="panel transfer-table"><div className="table-head"><span>FILE</span><span>ROUTE</span><span>DATE</span><span>STATUS</span><span /></div>{transferRows.filter(r => tab === "All transfers" || (tab === "Failed" ? r.status === "Failed" : tab === "Sending" ? r.from === "Windows PC" : r.to === "Windows PC")).map((r, i) => <div className="table-row" key={i}><div className="file-cell"><div className="file-icon"><File size={17} /></div><div><strong>{r.name}</strong><small>{r.size}</small></div></div><div className="route-cell"><span>{r.from}</span><ArrowRightIcon /><span>{r.to}</span></div><span className="date-cell">{r.time}</span><span className={`transfer-status ${r.status.toLowerCase().replace(" ", "-")}`}>{r.status}</span><button className="icon-button" onClick={() => notify(`${r.name} details`)}><MoreHorizontal size={17} /></button></div>)}</section>
    {showNew && <div className="modal-backdrop" onClick={() => setShowNew(false)}><div className="modal panel" onClick={e => e.stopPropagation()}><div className="modal-head"><div><div className="eyebrow">SEND A FILE</div><h2>New transfer</h2></div><button className="icon-button" onClick={() => setShowNew(false)}><X size={18} /></button></div><div className="drop-zone"><Upload size={25} /><strong>Drop files here</strong><span>or browse files from this device</span><button className="button secondary" onClick={() => notify("File picker is not connected in this UI preview")}>Choose files</button></div><label className="field-label">SEND TO</label><select className="select-field"><option>Android phone</option><option>Linux workstation</option><option>MacBook Air</option></select><div className="modal-actions"><button className="button secondary" onClick={() => setShowNew(false)}>Cancel</button><button className="button primary" onClick={() => { setShowNew(false); notify("Transfer UI is ready to connect to the backend"); }}>Continue <ChevronRight size={15} /></button></div></div></div>}
    <div className="privacy-footnote"><Lock size={14} /> Transfers are end-to-end encrypted. Cloud relay is used when direct peer-to-peer transfer isn't available.</div>
  </>;
}
function ArrowRightIcon() { return <ChevronRight size={13} className="route-arrow" />; }

function SecurityPage({ retention, setRetention, autoSync, setAutoSync, requireApproval, setRequireApproval, clipboardMonitoring, setClipboardMonitoring, notify }: { retention: string; setRetention: (s: string) => void; autoSync: boolean; setAutoSync: (b: boolean) => void; requireApproval: boolean; setRequireApproval: (b: boolean) => void; clipboardMonitoring: boolean; setClipboardMonitoring: (b: boolean) => void; notify: (s: string) => void }) {
  return <>
    <PageHeading eyebrow="YOUR PRIVACY, YOUR RULES" title="Security & privacy" description="Control how your data is protected, synced, and stored." />
    <div className="security-layout"><div className="security-main">
      <section className="panel settings-panel"><div className="settings-heading"><div className="settings-icon green"><ShieldCheck size={18} /></div><div><h2>Encryption</h2><p>Your data is encrypted before it leaves your device.</p></div><span className="verified-pill"><Check size={13} /> Active</span></div>
        <div className="setting-row"><div><strong>End-to-end encryption</strong><p>Only your trusted devices can decrypt synced content.</p></div><span className="locked-value"><Lock size={13} /> Always on</span></div>
        <div className="setting-row"><div><strong>Key management</strong><p>Encryption keys are generated and stored on your devices.</p></div><button className="text-link" onClick={() => notify("Key details view coming soon")}>View details <ChevronRight size={14} /></button></div>
      </section>
      <section className="panel settings-panel"><div className="settings-heading"><div className="settings-icon blue"><RefreshCw size={18} /></div><div><h2>Sync preferences</h2><p>Choose when and how your devices stay in sync.</p></div></div>
        <ToggleRow title="Automatic sync" description="Sync clipboard items when your trusted devices are online." checked={autoSync} onChange={setAutoSync} />
        <ToggleRow title="Ask before receiving" description="Require confirmation before accepting a new device's pairing request." checked={requireApproval} onChange={setRequireApproval} />
        <ToggleRow title="Clipboard monitoring" description="Allow the desktop app to watch clipboard changes while running. Disabled by default." checked={clipboardMonitoring} onChange={setClipboardMonitoring} />
      </section>
      <section className="panel settings-panel"><div className="settings-heading"><div className="settings-icon violet"><History size={18} /></div><div><h2>Clipboard history</h2><p>Set how long your local encrypted history is retained.</p></div></div>
        <div className="setting-row"><div><strong>Retention period</strong><p>Older items are automatically removed from this device.</p></div><select className="select-field compact-select" value={retention} onChange={e => setRetention(e.target.value)}><option>24 hours</option><option>7 days</option><option>30 days</option><option>90 days</option><option>Forever</option></select></div>
        <div className="setting-row"><div><strong>Clear clipboard history</strong><p>Remove all locally stored clipboard items from this device.</p></div><button className="button danger small" onClick={() => notify("History clear is a demo action")}>Clear history</button></div>
      </section>
    </div><aside className="security-aside">
      <div className="panel security-score"><div className="score-ring"><ShieldCheck size={26} /></div><div className="eyebrow">SECURITY OVERVIEW</div><h2>Looking good.</h2><p>Your ClipMesh setup follows the recommended security baseline.</p><div className="score-check"><Check size={14} /> End-to-end encryption enabled</div><div className="score-check"><Check size={14} /> Trusted devices only</div><div className="score-check"><Check size={14} /> Pairing approval enabled</div></div>
      <div className="panel key-card"><Fingerprint size={20} /><strong>Device identity</strong><p>Each device has its own cryptographic identity. Revoking a device blocks future access.</p><button className="text-link" onClick={() => notify("Device identity details coming soon")}>Learn about device keys <ChevronRight size={14} /></button></div>
    </aside></div>
    <div className="privacy-footnote"><Lock size={14} /> ClipMesh cannot read your clipboard content. This preview uses sample data and local UI state only.</div>
  </>;
}
function ToggleRow({ title, description, checked, onChange }: { title: string; description: string; checked: boolean; onChange: (b: boolean) => void }) {
  return <div className="setting-row"><div><strong>{title}</strong><p>{description}</p></div><button className={`toggle ${checked ? "on" : ""}`} role="switch" aria-checked={checked} onClick={() => onChange(!checked)}><span /></button></div>;
}

export default App;