export interface Account {
  username: string;
  password: string;
  rememberPassword: boolean;
  autoLogin: boolean;
}

export interface Preferences {
  closeBehavior: 'tray' | 'exit';
  noticeAcknowledged: boolean;
  autoCheckUpdates: boolean;
  guardIntervalSeconds: number;
}

export interface ClientState {
  revision: number;
  network: {
    state: 'online' | 'offline' | 'unknown';
    region: string;
    ip: string;
    account: string;
    mac: string;
    cvid: string;
    pvid: string;
    detail: string;
  };
  busy: boolean;
  guarding: boolean;
  username: string;
  savedUsername: string | null;
  rememberPassword: boolean;
  autoLogin: boolean;
  hasPassword: boolean;
  logGeneration: number;
  logCount: number;
  preferences: Preferences;
}

export interface UpdateState {
  revision: number;
  version: string;
  edition: string;
  message: string;
  failed: boolean;
  rollback: boolean;
  busy: boolean;
  downloading: boolean;
  bytes: number;
  prepared: boolean;
  release: null | { version: string; notes: string; page: string; size: number };
}

export interface View {
  client: ClientState;
  updater: UpdateState;
  trayReady: boolean;
}

export interface LogEntry {
  id: number;
  elapsedMs: number;
  level: 'INFO' | 'ERROR';
  message: string;
}
