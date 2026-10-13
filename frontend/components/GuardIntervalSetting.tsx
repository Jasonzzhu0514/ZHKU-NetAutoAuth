import { Button, Field, Input } from '@fluentui/react-components';
import { useEffect, useState } from 'react';
import type { Desktop } from '../useDesktop';

export function GuardIntervalSetting({ desktop }: { desktop: Desktop }) {
  const saved = desktop.view?.client.preferences.guardIntervalSeconds ?? 30;
  const [draft, setDraft] = useState(String(saved));
  useEffect(() => setDraft(String(saved)), [saved]);
  const seconds = Number(draft);
  const valid = draft.trim() !== '' && Number.isInteger(seconds) && seconds >= 30 && seconds <= 3600;
  return <section><h4>自动守护</h4>
    <Field label="检查间隔（30～3600 秒）" validationState={valid ? 'none' : 'error'}
      validationMessage={valid ? undefined : '请输入 30～3600 的整数'}>
      <div className="interval-setting">
        <Input aria-label="守护检查间隔" type="number" min={30} max={3600} step={1}
          contentAfter="秒" value={draft} disabled={!desktop.ready || desktop.executing}
          onChange={(_, data) => setDraft(data.value)} />
        <Button disabled={!desktop.ready || desktop.executing || !valid || seconds === saved}
          onClick={() => { void desktop.preferences({ guardIntervalSeconds: seconds }); }}>保存间隔</Button>
      </div>
    </Field>
  </section>;
}
