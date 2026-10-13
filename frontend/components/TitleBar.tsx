import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { DismissRegular, SquareMultipleRegular, SquareRegular, SubtractRegular, Wifi1Regular } from '@fluentui/react-icons';
import { useEffect, useState } from 'react';
import type { Desktop } from '../useDesktop';

export function TitleBar({ desktop }: { desktop: Desktop }) {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    if (!isTauri()) return;
    const window = getCurrentWindow();
    let alive = true;
    const update = () => { void window.isMaximized().then((value) => { if (alive) setMaximized(value); }).catch(() => {}); };
    update();
    const subscription = window.onResized(update);
    return () => { alive = false; void subscription.then((unlisten) => unlisten()); };
  }, []);
  const control = (action: 'minimize' | 'toggleMaximize' | 'close') => {
    if (isTauri()) void getCurrentWindow()[action]().catch((error) => desktop.setError(String(error)));
  };
  return <div className="window-titlebar">
    <div className="window-drag-area" data-tauri-drag-region>
      <Wifi1Regular /><span>ZHKU-NetAutoAuth</span>
    </div>
    <div className="window-controls">
      <button aria-label="最小化窗口" title="最小化" onClick={() => control('minimize')}><SubtractRegular /></button>
      <button aria-label={maximized ? '还原窗口' : '最大化窗口'} title={maximized ? '还原' : '最大化'}
        onClick={() => control('toggleMaximize')}>{maximized ? <SquareMultipleRegular /> : <SquareRegular />}</button>
      <button className="window-close" aria-label="关闭窗口" title="关闭" onClick={() => control('close')}><DismissRegular /></button>
    </div>
  </div>;
}
