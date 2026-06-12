import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Activity,
  Database,
  FolderOpen,
  Globe2,
  HardDrive,
  Play,
  Plus,
  Settings,
  TestTube2,
  X,
} from "lucide-react";
import "./App.css";

type AppPaths = {
  app_data_dir: string;
  database_path: string;
  config_path: string;
  credentials_path: string;
  profiles_dir: string;
  backups_dir: string;
};

type Profile = {
  id: string;
  name: string;
  notes: string;
  tags: string[];
  browser_binary_path: string | null;
  user_data_dir: string;
  proxy_id: string | null;
  proxy_scheme: string | null;
  proxy_host: string | null;
  proxy_port: number | null;
  proxy_username: string | null;
  proxy_password_saved: boolean;
  user_agent: string | null;
  language: string | null;
  timezone: string | null;
  profile_color: string | null;
  spoof_mac_address: string | null;
  randomize_mac_on_launch: boolean;
  webrtc_policy: string;
  webrtc_disabled: boolean;
  window_width: number | null;
  window_height: number | null;
  window_x: number | null;
  window_y: number | null;
  launch_args: string[];
  startup_urls: string[];
  created_at: string;
  updated_at: string;
  last_launched_at: string | null;
  running: boolean;
};

type BrowserCandidate = {
  name: string;
  app_path: string;
  binary_path: string;
  exists: boolean;
};

type ProxyProfile = {
  id: string;
  name: string;
  scheme: string;
  host: string;
  port: number;
  username: string | null;
  password_saved: boolean;
  notes: string;
  created_at: string;
  updated_at: string;
};

type ProxyTestResult = {
  ok: boolean;
  message: string;
  observed_ip: string | null;
};

type LaunchProfileResult = {
  profile_id: string;
  browser_binary_path: string;
  args: string[];
  launched_at: string;
};

type AppearanceBrowser = "chrome" | "firefox";
type AppearanceOs = "macos" | "windows" | "linux";
type ActiveTab = "profiles" | "proxies" | "settings";

type ProfileForm = {
  id: string | null;
  name: string;
  notes: string;
  tags: string;
  browserBinaryPath: string;
  proxyId: string;
  proxyEnabled: boolean;
  proxyScheme: string;
  proxyHost: string;
  proxyPort: string;
  proxyUsername: string;
  proxyPassword: string;
  proxyPasswordSaved: boolean;
  proxyUrl: string;
  appearanceBrowser: AppearanceBrowser;
  appearanceOs: AppearanceOs;
  userAgent: string;
  language: string;
  timezone: string;
  profileColor: string;
  spoofMacAddress: string;
  randomizeMacOnLaunch: boolean;
  webrtcPolicy: string;
  windowWidth: string;
  windowHeight: string;
  windowX: string;
  windowY: string;
  launchArgs: string;
  startupUrls: string;
};

type ProxyForm = {
  id: string | null;
  name: string;
  proxyUrl: string;
  scheme: string;
  host: string;
  port: string;
  username: string;
  password: string;
  passwordSaved: boolean;
  notes: string;
};

const defaultStartupUrls = "https://amiunique.org/fingerprint\nhttps://ifconfig.me";

const emptyForm: ProfileForm = {
  id: null,
  name: "",
  notes: "",
  tags: "",
  browserBinaryPath: "",
  proxyId: "",
  proxyEnabled: false,
  proxyScheme: "socks5",
  proxyHost: "127.0.0.1",
  proxyPort: "7891",
  proxyUsername: "",
  proxyPassword: "",
  proxyPasswordSaved: false,
  proxyUrl: "",
  appearanceBrowser: "chrome",
  appearanceOs: "macos",
  userAgent: "",
  language: "",
  timezone: "",
  profileColor: "#2563EB",
  spoofMacAddress: "",
  randomizeMacOnLaunch: false,
  webrtcPolicy: "proxy_only",
  windowWidth: "1280",
  windowHeight: "900",
  windowX: "80",
  windowY: "80",
  launchArgs: "",
  startupUrls: defaultStartupUrls,
};

const emptyProxyForm: ProxyForm = {
  id: null,
  name: "",
  proxyUrl: "",
  scheme: "socks5",
  host: "",
  port: "",
  username: "",
  password: "",
  passwordSaved: false,
  notes: "",
};

const languageOptions = [
  { value: "en-US", label: "English (US)" },
  { value: "en-GB", label: "English (UK)" },
  { value: "ja-JP", label: "Japanese" },
  { value: "zh-CN", label: "Chinese (Simplified)" },
  { value: "zh-TW", label: "Chinese (Traditional)" },
  { value: "ko-KR", label: "Korean" },
  { value: "fr-FR", label: "French" },
  { value: "de-DE", label: "German" },
  { value: "es-ES", label: "Spanish" },
];

const timezoneOptions = [
  "America/Los_Angeles",
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "Europe/London",
  "Europe/Paris",
  "Europe/Berlin",
  "Asia/Tokyo",
  "Asia/Shanghai",
  "Asia/Singapore",
  "Asia/Seoul",
  "Australia/Sydney",
  "UTC",
];

const profileColorPalette = [
  "#2563EB",
  "#059669",
  "#DC2626",
  "#7C3AED",
  "#D97706",
  "#0F766E",
  "#DB2777",
  "#4F46E5",
  "#65A30D",
  "#0891B2",
];

function randomProfileColor() {
  return profileColorPalette[Math.floor(Math.random() * profileColorPalette.length)];
}

function randomSpoofMacAddress() {
  const bytes = crypto.getRandomValues(new Uint8Array(6));
  bytes[0] = (bytes[0] & 0xfc) | 0x02;
  return Array.from(bytes)
    .map((byte) => byte.toString(16).padStart(2, "0").toUpperCase())
    .join(":");
}

function formatDeviceTime(value: string | null) {
  if (!value) {
    return "Never";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

function profileRoute(profile: Profile) {
  if (profile.proxy_scheme) {
    return `${profile.proxy_scheme}://${profile.proxy_host}:${profile.proxy_port}`;
  }
  if (profile.browser_binary_path) {
    return "Custom";
  }
  return "Detected";
}

const userAgentPresets: Record<AppearanceBrowser, Record<AppearanceOs, string>> = {
  chrome: {
    macos:
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.178 Safari/537.36",
    windows:
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.178 Safari/537.36",
    linux:
      "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.7680.177 Safari/537.36",
  },
  firefox: {
    macos: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:149.0) Gecko/20100101 Firefox/149.0",
    windows: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
    linux: "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0",
  },
};

function userAgentPreset(browser: AppearanceBrowser, os: AppearanceOs) {
  return userAgentPresets[browser][os];
}

function defaultWindowForOs(os: AppearanceOs) {
  if (os === "windows") {
    return { width: "1366", height: "768" };
  }
  if (os === "linux") {
    return { width: "1440", height: "900" };
  }
  return { width: "1280", height: "900" };
}

function parseProxyUrlParts(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  try {
    const parsed = new URL(trimmed);
    if (parsed.hostname && parsed.port) {
      return {
        scheme: parsed.protocol.replace(":", "") || "socks5",
        host: parsed.hostname,
        port: parsed.port,
        username: decodeURIComponent(parsed.username),
        password: decodeURIComponent(parsed.password),
      };
    }
  } catch {
    // Some providers copy non-standard socks5://host:port:user:pass strings.
  }

  const withoutScheme = trimmed.replace(/^[a-z0-9]+:\/\//i, "");
  const scheme = trimmed.includes("://") ? trimmed.split("://")[0].toLowerCase() : "socks5";
  const [host, port, username, ...passwordParts] = withoutScheme.split(":");
  const password = passwordParts.join(":");
  if (host && port) {
    return { scheme, host, port, username: username || "", password };
  }

  return null;
}

function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>("profiles");
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [browsers, setBrowsers] = useState<BrowserCandidate[]>([]);
  const [proxyProfiles, setProxyProfiles] = useState<ProxyProfile[]>([]);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [form, setForm] = useState<ProfileForm>(emptyForm);
  const [proxyForm, setProxyForm] = useState<ProxyForm>(emptyProxyForm);
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [isProxyEditorOpen, setIsProxyEditorOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [launchMessage, setLaunchMessage] = useState<string | null>(null);
  const [proxyTestMessage, setProxyTestMessage] = useState<string | null>(null);

  async function startWindowDrag(event: React.MouseEvent<HTMLElement>) {
    if (event.button !== 0) {
      return;
    }
    try {
      await getCurrentWindow().startDragging();
    } catch {
      // Browser preview cannot drag a native Tauri window.
    }
  }

  async function load() {
    try {
      const [appPaths, browserRows, proxyRows, profileRows] = await Promise.all([
        invoke<AppPaths>("get_app_paths"),
        invoke<BrowserCandidate[]>("detect_browsers"),
        invoke<ProxyProfile[]>("list_proxy_profiles"),
        invoke<Profile[]>("list_profiles"),
      ]);
      setPaths(appPaths);
      setBrowsers(browserRows);
      setProxyProfiles(proxyRows);
      setProfiles(profileRows);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  useEffect(() => {
    load();
  }, []);

  function splitLines(value: string) {
    return value
      .split("\n")
      .map((item) => item.trim())
      .filter(Boolean);
  }

  function splitTags(value: string) {
    return value
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
  }

  function optionalNumber(value: string) {
    const trimmed = value.trim();
    return trimmed ? Number(trimmed) : null;
  }

  function parseProxyUrl(value: string) {
    const parsed = parseProxyUrlParts(value);
    if (parsed) {
      setForm({
        ...form,
        proxyEnabled: true,
        proxyId: "",
        proxyScheme: parsed.scheme,
        proxyHost: parsed.host,
        proxyPort: parsed.port,
        proxyUsername: parsed.username,
        proxyPassword: parsed.password,
        proxyUrl: "",
      });
    }
  }

  function parseProxyProfileUrl(value: string) {
    const parsed = parseProxyUrlParts(value);
    if (parsed) {
      setProxyForm({
        ...proxyForm,
        scheme: parsed.scheme,
        host: parsed.host,
        port: parsed.port,
        username: parsed.username,
        password: parsed.password,
        proxyUrl: "",
      });
    }
  }

  function openNewProfile() {
    setForm({
      ...emptyForm,
      profileColor: randomProfileColor(),
      spoofMacAddress: randomSpoofMacAddress(),
    });
    setIsEditorOpen(true);
  }

  function openExistingProfile(profile: Profile) {
    setForm({
      id: profile.id,
      name: profile.name,
      notes: profile.notes,
      tags: profile.tags.join(", "),
      browserBinaryPath: profile.browser_binary_path ?? "",
      proxyId: profile.proxy_id ?? "",
      proxyEnabled: Boolean(profile.proxy_scheme && profile.proxy_host && profile.proxy_port),
      proxyScheme: profile.proxy_scheme ?? "socks5",
      proxyHost: profile.proxy_host ?? "127.0.0.1",
      proxyPort: profile.proxy_port?.toString() ?? "7891",
      proxyUsername: profile.proxy_username ?? "",
      proxyPassword: "",
      proxyPasswordSaved: profile.proxy_password_saved,
      proxyUrl: "",
      appearanceBrowser: "chrome",
      appearanceOs: "macos",
      userAgent: profile.user_agent ?? "",
      language: profile.language ?? "",
      timezone: profile.timezone ?? "",
      profileColor: profile.profile_color ?? randomProfileColor(),
      spoofMacAddress: profile.spoof_mac_address ?? randomSpoofMacAddress(),
      randomizeMacOnLaunch: profile.randomize_mac_on_launch,
      webrtcPolicy: profile.webrtc_policy || (profile.webrtc_disabled ? "proxy_only" : "default"),
      windowWidth: profile.window_width?.toString() ?? "1280",
      windowHeight: profile.window_height?.toString() ?? "900",
      windowX: profile.window_x?.toString() ?? "80",
      windowY: profile.window_y?.toString() ?? "80",
      launchArgs: profile.launch_args.join("\n"),
      startupUrls: profile.startup_urls.join("\n"),
    });
    setIsEditorOpen(true);
  }

  function selectProxyProfile(proxyId: string) {
    const proxy = proxyProfiles.find((item) => item.id === proxyId);
    if (!proxy) {
      setForm({
        ...form,
        proxyId: "",
        proxyEnabled: false,
      });
      return;
    }
    setForm({
      ...form,
      proxyId: proxy.id,
      proxyEnabled: true,
      proxyScheme: proxy.scheme,
      proxyHost: proxy.host,
      proxyPort: proxy.port.toString(),
      proxyUsername: proxy.username ?? "",
      proxyPassword: "",
      proxyPasswordSaved: proxy.password_saved,
    });
  }

  function openNewProxyProfile() {
    setProxyForm(emptyProxyForm);
    setIsProxyEditorOpen(true);
  }

  function openExistingProxyProfile(proxy: ProxyProfile) {
    setProxyForm({
      id: proxy.id,
      name: proxy.name,
      proxyUrl: "",
      scheme: proxy.scheme,
      host: proxy.host,
      port: proxy.port.toString(),
      username: proxy.username ?? "",
      password: "",
      passwordSaved: proxy.password_saved,
      notes: proxy.notes,
    });
    setIsProxyEditorOpen(true);
  }

  function applyAppearancePreset() {
    const windowSize = defaultWindowForOs(form.appearanceOs);
    setForm({
      ...form,
      userAgent: userAgentPreset(form.appearanceBrowser, form.appearanceOs),
      language: form.language || "en-US",
      timezone: form.timezone || (form.appearanceOs === "windows" ? "America/Los_Angeles" : ""),
      windowWidth: windowSize.width,
      windowHeight: windowSize.height,
    });
  }

  async function saveProfile(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const payload = {
      name: form.name,
      notes: form.notes,
      tags: splitTags(form.tags),
      browser_binary_path: form.browserBinaryPath.trim() || null,
      proxy_id: form.proxyId || null,
      proxy_scheme: form.proxyEnabled ? form.proxyScheme : null,
      proxy_host: form.proxyEnabled ? form.proxyHost : null,
      proxy_port: form.proxyEnabled ? Number(form.proxyPort) : null,
      proxy_username: form.proxyEnabled ? form.proxyUsername : null,
      proxy_password: form.proxyEnabled && form.proxyPassword ? form.proxyPassword : null,
      user_agent: form.userAgent.trim() || null,
      language: form.language.trim() || null,
      timezone: form.timezone.trim() || null,
      profile_color: form.profileColor.trim() || null,
      spoof_mac_address: form.spoofMacAddress.trim() || null,
      randomize_mac_on_launch: form.randomizeMacOnLaunch,
      webrtc_policy: form.webrtcPolicy,
      webrtc_disabled: form.webrtcPolicy !== "default",
      window_width: optionalNumber(form.windowWidth),
      window_height: optionalNumber(form.windowHeight),
      window_x: optionalNumber(form.windowX),
      window_y: optionalNumber(form.windowY),
      launch_args: splitLines(form.launchArgs),
      startup_urls: splitLines(form.startupUrls),
    };

    try {
      if (form.id) {
        await invoke("update_profile", { id: form.id, input: payload });
      } else {
        await invoke("create_profile", { input: payload });
      }
      setForm(emptyForm);
      setIsEditorOpen(false);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function deleteSelectedProfile() {
    if (!form.id) {
      return;
    }

    try {
      await invoke("delete_profile", { id: form.id, deleteFiles: false });
      setForm(emptyForm);
      setIsEditorOpen(false);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function saveProxyProfile(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const payload = {
      name: proxyForm.name,
      scheme: proxyForm.scheme,
      host: proxyForm.host,
      port: Number(proxyForm.port),
      username: proxyForm.username.trim() || null,
      password: proxyForm.password || null,
      notes: proxyForm.notes,
    };

    try {
      if (proxyForm.id) {
        await invoke("update_proxy_profile", { id: proxyForm.id, input: payload });
      } else {
        await invoke("create_proxy_profile", { input: payload });
      }
      setProxyForm(emptyProxyForm);
      setIsProxyEditorOpen(false);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function deleteSelectedProxyProfile() {
    if (!proxyForm.id) {
      return;
    }

    try {
      await invoke("delete_proxy_profile", { id: proxyForm.id });
      setProxyForm(emptyProxyForm);
      setIsProxyEditorOpen(false);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function launchProfile(profileId: string) {
    try {
      const result = await invoke<LaunchProfileResult>("launch_profile", { id: profileId });
      const proxyArg = result.args.find((arg) => arg.startsWith("--proxy-server="));
      setLaunchMessage(
        proxyArg
          ? `Launched profile through ${proxyArg.replace("--proxy-server=", "")}.`
          : "Launched profile without an explicit proxy flag.",
      );
      setProxyTestMessage(null);
      setError(null);
      await load();
    } catch (err) {
      setLaunchMessage(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function closeProfile(profileId: string) {
    try {
      const pids = await invoke<number[]>("close_profile", { id: profileId });
      setLaunchMessage(
        pids.length
          ? `Closed profile browser processes: ${pids.join(", ")}.`
          : "No running browser processes found for this profile.",
      );
      setProxyTestMessage(null);
      setError(null);
      await load();
    } catch (err) {
      setLaunchMessage(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function testProfileProxy(profileId: string) {
    try {
      const result = await invoke<ProxyTestResult>("test_profile_proxy", { id: profileId });
      setProxyTestMessage(
        result.ok
          ? `Proxy test passed. Observed IP: ${result.observed_ip ?? "unknown"}. ${result.message}`
          : `Proxy test failed. ${result.message}`,
      );
      setError(null);
    } catch (err) {
      setProxyTestMessage(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  const detectedBrowser = browsers.find((browser) => browser.exists);

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div
          className="sidebar-brand drag-surface"
          data-tauri-drag-region=""
          onMouseDown={startWindowDrag}
        >
          <strong>FingerBrow</strong>
          <span>Drag here</span>
        </div>
        <nav aria-label="Primary">
          <button
            className={`nav-item ${activeTab === "profiles" ? "active" : ""}`}
            type="button"
            onClick={() => setActiveTab("profiles")}
          >
            <Activity size={18} strokeWidth={1.8} />
            Profiles
          </button>
          <button
            className={`nav-item ${activeTab === "proxies" ? "active" : ""}`}
            type="button"
            onClick={() => setActiveTab("proxies")}
          >
            <Globe2 size={18} strokeWidth={1.8} />
            Proxies
          </button>
          <button
            className={`nav-item ${activeTab === "settings" ? "active" : ""}`}
            type="button"
            onClick={() => setActiveTab("settings")}
          >
            <Settings size={18} strokeWidth={1.8} />
            Settings
          </button>
        </nav>
      </aside>

      <section className="workspace">
        <div
          className="drag-bar"
          data-tauri-drag-region=""
          onMouseDown={startWindowDrag}
          role="presentation"
        >
          <span>Drag window</span>
        </div>
        <header className="toolbar">
          <div>
            <h2>
              {activeTab === "proxies"
                ? "Proxies"
                : activeTab === "settings"
                  ? "Settings"
                  : "Profiles"}
            </h2>
            <p>
              {activeTab === "proxies"
                ? `${proxyProfiles.length} proxy profiles`
                : activeTab === "settings"
                  ? "Local app paths"
                  : `${profiles.length} local profiles`}
            </p>
          </div>
          {activeTab === "profiles" ? (
            <button className="primary-button" type="button" onClick={openNewProfile}>
              <Plus size={17} strokeWidth={2} />
              New Profile
            </button>
          ) : null}
          {activeTab === "proxies" ? (
            <button className="primary-button" type="button" onClick={openNewProxyProfile}>
              <Plus size={17} strokeWidth={2} />
              New Proxy
            </button>
          ) : null}
        </header>

        {error ? <div className="notice error">{error}</div> : null}
        {launchMessage ? <div className="notice success">{launchMessage}</div> : null}
        {proxyTestMessage ? <div className="notice success">{proxyTestMessage}</div> : null}

        {activeTab !== "proxies" ? (
          <section className="system-panel" aria-label="Local storage paths">
            <div>
              <span>
                <FolderOpen size={15} strokeWidth={1.8} />
                App data
              </span>
              <strong>{paths?.app_data_dir ?? "Loading..."}</strong>
            </div>
            <div>
              <span>
                <Database size={15} strokeWidth={1.8} />
                Database
              </span>
              <strong>{paths?.database_path ?? "Loading..."}</strong>
            </div>
            <div>
              <span>
                <HardDrive size={15} strokeWidth={1.8} />
                Detected browser
              </span>
              <strong>
                {detectedBrowser?.binary_path ?? "No Chrome/Chromium binary detected"}
              </strong>
            </div>
          </section>
        ) : null}

        {activeTab === "profiles" && isEditorOpen ? (
          <form className="editor-panel" onSubmit={saveProfile}>
            <header>
              <h3>{form.id ? "Edit Profile" : "New Profile"}</h3>
              <button type="button" onClick={() => setIsEditorOpen(false)}>
                Close
              </button>
            </header>
            <label>
              <span>Name</span>
              <input
                autoFocus
                required
                value={form.name}
                onChange={(event) => setForm({ ...form, name: event.currentTarget.value })}
              />
            </label>
            <label>
              <span>Tags</span>
              <input
                placeholder="work, qa, banking"
                value={form.tags}
                onChange={(event) => setForm({ ...form, tags: event.currentTarget.value })}
              />
            </label>
            <div className="color-setting">
              <label>
                <span>Profile Color</span>
                <input
                  type="color"
                  value={form.profileColor}
                  onChange={(event) =>
                    setForm({ ...form, profileColor: event.currentTarget.value })
                  }
                />
              </label>
              <div className="color-swatches" aria-label="Profile color presets">
                {profileColorPalette.map((color) => (
                  <button
                    aria-label={`Use ${color}`}
                    className={form.profileColor.toUpperCase() === color ? "selected" : ""}
                    key={color}
                    style={{ background: color }}
                    type="button"
                    onClick={() => setForm({ ...form, profileColor: color })}
                  />
                ))}
                <button
                  type="button"
                  onClick={() => setForm({ ...form, profileColor: randomProfileColor() })}
                >
                  Random
                </button>
              </div>
            </div>
            <label>
              <span>Browser Binary</span>
              <input
                placeholder={detectedBrowser?.binary_path ?? "/Applications/Google Chrome.app/..."}
                value={form.browserBinaryPath}
                onChange={(event) =>
                  setForm({ ...form, browserBinaryPath: event.currentTarget.value })
                }
              />
            </label>
            <fieldset className="proxy-fields">
              <legend>Profile Proxy</legend>
              <label>
                <span>Saved Proxy</span>
                <select
                  value={form.proxyId}
                  onChange={(event) => selectProxyProfile(event.currentTarget.value)}
                >
                  <option value="">Manual / none</option>
                  {proxyProfiles.map((proxy) => (
                    <option key={proxy.id} value={proxy.id}>
                      {proxy.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>Paste Proxy URL</span>
                <input
                  disabled={Boolean(form.proxyId)}
                  placeholder="socks5://host:port:username:password"
                  value={form.proxyUrl}
                  onBlur={(event) => parseProxyUrl(event.currentTarget.value)}
                  onChange={(event) => setForm({ ...form, proxyUrl: event.currentTarget.value })}
                />
              </label>
              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={form.proxyEnabled}
                  disabled={Boolean(form.proxyId)}
                  onChange={(event) =>
                    setForm({ ...form, proxyEnabled: event.currentTarget.checked })
                  }
                />
                <span>Use explicit proxy for this profile</span>
              </label>
              <div className="proxy-grid">
                <label>
                  <span>Type</span>
                  <select
                    disabled={!form.proxyEnabled || Boolean(form.proxyId)}
                    value={form.proxyScheme}
                    onChange={(event) =>
                      setForm({ ...form, proxyScheme: event.currentTarget.value })
                    }
                  >
                    <option value="socks5">SOCKS5</option>
                    <option value="socks4">SOCKS4</option>
                    <option value="http">HTTP</option>
                    <option value="https">HTTPS</option>
                  </select>
                </label>
                <label>
                  <span>Host</span>
                  <input
                    disabled={!form.proxyEnabled || Boolean(form.proxyId)}
                    value={form.proxyHost}
                    onChange={(event) => setForm({ ...form, proxyHost: event.currentTarget.value })}
                  />
                </label>
                <label>
                  <span>Port</span>
                  <input
                    disabled={!form.proxyEnabled || Boolean(form.proxyId)}
                    inputMode="numeric"
                    pattern="[0-9]*"
                    value={form.proxyPort}
                    onChange={(event) => setForm({ ...form, proxyPort: event.currentTarget.value })}
                  />
                </label>
                <label>
                  <span>Username</span>
                  <input
                    disabled={!form.proxyEnabled || Boolean(form.proxyId)}
                    value={form.proxyUsername}
                    onChange={(event) =>
                      setForm({ ...form, proxyUsername: event.currentTarget.value })
                    }
                  />
                </label>
              </div>
              <label>
                <span>Password</span>
                <input
                  disabled={!form.proxyEnabled || Boolean(form.proxyId)}
                  placeholder={form.proxyPasswordSaved ? "Saved in Keychain" : ""}
                  type="password"
                  value={form.proxyPassword}
                  onChange={(event) =>
                    setForm({ ...form, proxyPassword: event.currentTarget.value })
                  }
                />
              </label>
            </fieldset>
            <fieldset className="proxy-fields">
              <legend>Browser Differences</legend>
              <div className="preset-grid">
                <label>
                  <span>Browser</span>
                  <select
                    value={form.appearanceBrowser}
                    onChange={(event) =>
                      setForm({
                        ...form,
                        appearanceBrowser: event.currentTarget.value as AppearanceBrowser,
                      })
                    }
                  >
                    <option value="chrome">Chrome</option>
                    <option value="firefox">Firefox</option>
                  </select>
                </label>
                <label>
                  <span>Operating System</span>
                  <select
                    value={form.appearanceOs}
                    onChange={(event) =>
                      setForm({ ...form, appearanceOs: event.currentTarget.value as AppearanceOs })
                    }
                  >
                    <option value="macos">macOS</option>
                    <option value="windows">Windows</option>
                    <option value="linux">Linux</option>
                  </select>
                </label>
                <button type="button" onClick={applyAppearancePreset}>
                  Apply Preset
                </button>
              </div>
              <label>
                <span>User-Agent</span>
                <textarea
                  value={form.userAgent}
                  onChange={(event) => setForm({ ...form, userAgent: event.currentTarget.value })}
                />
              </label>
              <div className="proxy-grid">
                <label>
                  <span>Language</span>
                  <select
                    value={form.language}
                    onChange={(event) => setForm({ ...form, language: event.currentTarget.value })}
                  >
                    <option value="">System default</option>
                    {languageOptions.map((language) => (
                      <option key={language.value} value={language.value}>
                        {language.label}
                      </option>
                    ))}
                    {form.language &&
                    !languageOptions.some((language) => language.value === form.language) ? (
                      <option value={form.language}>{form.language}</option>
                    ) : null}
                  </select>
                </label>
                <label>
                  <span>Timezone</span>
                  <select
                    value={form.timezone}
                    onChange={(event) => setForm({ ...form, timezone: event.currentTarget.value })}
                  >
                    <option value="">System default</option>
                    {timezoneOptions.map((timezone) => (
                      <option key={timezone} value={timezone}>
                        {timezone}
                      </option>
                    ))}
                    {form.timezone && !timezoneOptions.includes(form.timezone) ? (
                      <option value={form.timezone}>{form.timezone}</option>
                    ) : null}
                  </select>
                </label>
                <label>
                  <span>Width</span>
                  <input
                    inputMode="numeric"
                    value={form.windowWidth}
                    onChange={(event) =>
                      setForm({ ...form, windowWidth: event.currentTarget.value })
                    }
                  />
                </label>
                <label>
                  <span>Height</span>
                  <input
                    inputMode="numeric"
                    value={form.windowHeight}
                    onChange={(event) =>
                      setForm({ ...form, windowHeight: event.currentTarget.value })
                    }
                  />
                </label>
                <label>
                  <span>X</span>
                  <input
                    inputMode="numeric"
                    value={form.windowX}
                    onChange={(event) => setForm({ ...form, windowX: event.currentTarget.value })}
                  />
                </label>
                <label>
                  <span>Y</span>
                  <input
                    inputMode="numeric"
                    value={form.windowY}
                    onChange={(event) => setForm({ ...form, windowY: event.currentTarget.value })}
                  />
                </label>
              </div>
              <label>
                <span>WebRTC Policy</span>
                <select
                  value={form.webrtcPolicy}
                  onChange={(event) =>
                    setForm({ ...form, webrtcPolicy: event.currentTarget.value })
                  }
                >
                  <option value="proxy_only">Use proxy route only</option>
                  <option value="public_only">Public interface only</option>
                  <option value="default">Chrome default</option>
                </select>
              </label>
              <div className="mac-grid">
                <label>
                  <span>Spoof MAC</span>
                  <input
                    value={form.spoofMacAddress}
                    onChange={(event) =>
                      setForm({ ...form, spoofMacAddress: event.currentTarget.value })
                    }
                  />
                </label>
                <button
                  type="button"
                  onClick={() => setForm({ ...form, spoofMacAddress: randomSpoofMacAddress() })}
                >
                  Random
                </button>
              </div>
              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={form.randomizeMacOnLaunch}
                  onChange={(event) =>
                    setForm({ ...form, randomizeMacOnLaunch: event.currentTarget.checked })
                  }
                />
                <span>Randomize spoof MAC on launch</span>
              </label>
            </fieldset>
            <label>
              <span>Launch Args</span>
              <textarea
                value={form.launchArgs}
                onChange={(event) => setForm({ ...form, launchArgs: event.currentTarget.value })}
              />
            </label>
            <label>
              <span>Startup URLs</span>
              <textarea
                value={form.startupUrls}
                onChange={(event) => setForm({ ...form, startupUrls: event.currentTarget.value })}
              />
            </label>
            <label>
              <span>Notes</span>
              <textarea
                value={form.notes}
                onChange={(event) => setForm({ ...form, notes: event.currentTarget.value })}
              />
            </label>
            <footer>
              {form.id ? (
                <button className="danger" type="button" onClick={deleteSelectedProfile}>
                  Delete
                </button>
              ) : null}
              <button type="submit">Save Profile</button>
            </footer>
          </form>
        ) : null}

        {activeTab === "profiles" ? (
          <section className="profile-table" aria-label="Profiles">
            <div className="table-row table-head">
              <span>Name</span>
              <span>Tags</span>
              <span>MAC</span>
              <span>Route</span>
              <span>Last launched</span>
              <span>Actions</span>
            </div>

            {profiles.length === 0 ? (
              <div className="empty-state">No profiles yet.</div>
            ) : (
              profiles.map((profile) => {
                const route = profileRoute(profile);
                return (
                  <div className="table-row profile-row" key={profile.id}>
                    <button
                      className="link-button"
                      type="button"
                      onClick={() => openExistingProfile(profile)}
                    >
                      <span
                        className="profile-color-dot"
                        style={{ backgroundColor: profile.profile_color ?? "#94A3B8" }}
                      />
                      <span>{profile.name}</span>
                      {profile.running ? <span className="running-badge">Open</span> : null}
                    </button>
                    <span>{profile.tags.join(", ") || "None"}</span>
                    <span className="mac-cell" title={profile.spoof_mac_address ?? ""}>
                      {profile.spoof_mac_address ?? "Auto"}
                    </span>
                    <span className="route-cell route-with-test">
                      <span className="route-text" title={route}>
                        {route}
                      </span>
                      <button type="button" onClick={() => testProfileProxy(profile.id)}>
                        <TestTube2 size={15} strokeWidth={1.9} />
                        Test
                      </button>
                    </span>
                    <span className="date-cell" title={profile.last_launched_at ?? ""}>
                      {formatDeviceTime(profile.last_launched_at)}
                    </span>
                    <span className="action-cell">
                      <button type="button" onClick={() => launchProfile(profile.id)}>
                        <Play size={15} strokeWidth={1.9} />
                        Launch
                      </button>
                      <button type="button" onClick={() => closeProfile(profile.id)}>
                        <X size={15} strokeWidth={1.9} />
                        Close
                      </button>
                    </span>
                  </div>
                );
              })
            )}
          </section>
        ) : null}

        {activeTab === "proxies" && isProxyEditorOpen ? (
          <form className="editor-panel" onSubmit={saveProxyProfile}>
            <header>
              <h3>{proxyForm.id ? "Edit Proxy" : "New Proxy"}</h3>
              <button type="button" onClick={() => setIsProxyEditorOpen(false)}>
                Close
              </button>
            </header>
            <label>
              <span>Name</span>
              <input
                required
                value={proxyForm.name}
                onChange={(event) =>
                  setProxyForm({ ...proxyForm, name: event.currentTarget.value })
                }
              />
            </label>
            <label>
              <span>Paste Proxy URL</span>
              <input
                placeholder="socks5://host:port:username:password"
                value={proxyForm.proxyUrl}
                onBlur={(event) => parseProxyProfileUrl(event.currentTarget.value)}
                onChange={(event) =>
                  setProxyForm({ ...proxyForm, proxyUrl: event.currentTarget.value })
                }
              />
            </label>
            <div className="proxy-grid">
              <label>
                <span>Type</span>
                <select
                  value={proxyForm.scheme}
                  onChange={(event) =>
                    setProxyForm({ ...proxyForm, scheme: event.currentTarget.value })
                  }
                >
                  <option value="socks5">SOCKS5</option>
                  <option value="socks4">SOCKS4</option>
                  <option value="http">HTTP</option>
                  <option value="https">HTTPS</option>
                </select>
              </label>
              <label>
                <span>Host</span>
                <input
                  required
                  value={proxyForm.host}
                  onChange={(event) =>
                    setProxyForm({ ...proxyForm, host: event.currentTarget.value })
                  }
                />
              </label>
              <label>
                <span>Port</span>
                <input
                  required
                  inputMode="numeric"
                  value={proxyForm.port}
                  onChange={(event) =>
                    setProxyForm({ ...proxyForm, port: event.currentTarget.value })
                  }
                />
              </label>
              <label>
                <span>Username</span>
                <input
                  value={proxyForm.username}
                  onChange={(event) =>
                    setProxyForm({ ...proxyForm, username: event.currentTarget.value })
                  }
                />
              </label>
            </div>
            <label>
              <span>Password</span>
              <input
                placeholder={proxyForm.passwordSaved ? "Saved in Keychain" : ""}
                type="password"
                value={proxyForm.password}
                onChange={(event) =>
                  setProxyForm({ ...proxyForm, password: event.currentTarget.value })
                }
              />
            </label>
            <label>
              <span>Notes</span>
              <textarea
                value={proxyForm.notes}
                onChange={(event) =>
                  setProxyForm({ ...proxyForm, notes: event.currentTarget.value })
                }
              />
            </label>
            <footer>
              {proxyForm.id ? (
                <button className="danger" type="button" onClick={deleteSelectedProxyProfile}>
                  Delete
                </button>
              ) : null}
              <button type="submit">Save Proxy</button>
            </footer>
          </form>
        ) : null}

        {activeTab === "proxies" ? (
          <section className="profile-table" aria-label="Proxy profiles">
            <div className="table-row proxy-table-head">
              <span>Name</span>
              <span>Route</span>
              <span>Auth</span>
              <span>Actions</span>
            </div>
            {proxyProfiles.length === 0 ? (
              <div className="empty-state">No proxy profiles yet.</div>
            ) : (
              proxyProfiles.map((proxy) => (
                <div className="table-row proxy-row" key={proxy.id}>
                  <button
                    className="link-button"
                    type="button"
                    onClick={() => openExistingProxyProfile(proxy)}
                  >
                    {proxy.name}
                  </button>
                  <span className="route-cell">{`${proxy.scheme}://${proxy.host}:${proxy.port}`}</span>
                  <span>{proxy.username ? "Username + Keychain" : "No auth"}</span>
                  <span className="action-cell">
                    <button type="button" onClick={() => openExistingProxyProfile(proxy)}>
                      Edit
                    </button>
                  </span>
                </div>
              ))
            )}
          </section>
        ) : null}
      </section>
    </main>
  );
}

export default App;
