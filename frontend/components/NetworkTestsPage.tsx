import { Badge, Button, Card, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface, DialogTitle, Field, Input, MessageBar, MessageBarBody, Spinner } from '@fluentui/react-components';
import { AddRegular, ArrowSyncRegular, DeleteRegular, GlobeRegular } from '@fluentui/react-icons';
import { useState } from 'react';
import { logTime } from '../logTime';
import { useNetworkTests } from '../useNetworkTests';
import './networkTests.css';

export function NetworkTestsPage({ active }: { active: boolean }) {
  const tests = useNetworkTests(active);
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');
  const entries = tests.snapshot?.entries || [];
  const busy = !tests.ready || tests.executing || !!tests.snapshot?.busy;
  const add = async () => {
    if (await tests.run('add', { name, url })) { setAdding(false); setName(''); setUrl(''); }
  };
  return <>
    <Card className="website-test-toolbar">
      <div><h3>网站连接延迟</h3><p>测试结果仅反映连接延迟，可能与网页加载速度不同。</p></div>
      <div className="website-test-actions">
        <Button icon={<AddRegular />} disabled={busy || entries.length >= 32} onClick={() => { tests.setError(''); setAdding(true); }}>添加网站</Button>
        <Button appearance="primary" icon={tests.snapshot?.busy ? <Spinner size="tiny" /> : <ArrowSyncRegular />}
          disabled={busy || !entries.length} onClick={() => { void tests.run('test'); }}>测试全部</Button>
      </div>
    </Card>
    {!adding && tests.error && <MessageBar intent="error"><MessageBarBody>{tests.error}</MessageBarBody></MessageBar>}
    {tests.snapshot?.warning && <MessageBar intent="warning"><MessageBarBody>{tests.snapshot.warning}</MessageBarBody></MessageBar>}
    <div className="website-test-list" aria-label="网站列表">
      {!tests.ready && !entries.length && <div className="website-test-empty">正在读取网站列表…</div>}
      {tests.ready && !entries.length && <div className="website-test-empty">暂无网站，点击「添加网站」开始。</div>}
      {entries.map(({ target, result }) => {
        const pending = tests.snapshot?.pending.includes(target.id);
        const time = result ? logTime(result.testedAtMs) : null;
        return <Card className="website-test-card" key={target.id} aria-label={`${target.name}测试结果`}>
          <div className="website-test-heading"><GlobeRegular aria-hidden="true" />
            <div className="website-test-identity"><strong>{target.name}</strong><span title={target.url}>{target.url}</span></div>
            <Button appearance="subtle" icon={<DeleteRegular />} aria-label={`删除${target.name}`} disabled={busy}
              onClick={() => { void tests.run('remove', { id: target.id }); }} />
          </div>
          <div className="website-test-result">
            <strong className={result?.failure ? 'error-text' : result ? 'website-test-success' : ''}>
              {result?.latencyMs != null ? <>{result.latencyMs}<span> ms</span></> : result ? '未连接' : '—'}
            </strong>
            {pending ? <Badge appearance="tint" color="brand">测试中</Badge>
              : result ? <Badge appearance="tint" color={result.failure ? 'danger' : 'success'}>{result.failure ? '连接失败' : '连接成功'}</Badge>
              : <Badge appearance="tint" color="subtle">尚未测试</Badge>}
          </div>
          <p className={`website-test-reason ${result?.failure ? 'error-text' : ''}`}>
            {pending ? '正在建立连接…' : result?.failure || (result ? '' : '点击测试获取连接延迟')}
          </p>
          <div className="website-test-footer"><time dateTime={time?.iso}>{time ? `最近测试 ${time.text}` : '最近测试 —'}</time>
            <Button size="small" icon={pending ? <Spinner size="tiny" /> : <ArrowSyncRegular />} disabled={busy}
              aria-label={`测试${target.name}`} onClick={() => { void tests.run('test', { id: target.id }); }}>测试</Button>
          </div>
        </Card>;
      })}
    </div>
    <Dialog open={adding} onOpenChange={(_, data) => { if (!tests.executing) { setAdding(data.open); if (!data.open) tests.setError(''); } }}>
      <DialogSurface aria-label="添加网站"><DialogBody><DialogTitle>添加网站</DialogTitle>
        <DialogContent><div className="website-test-form">
          <Field label="网站名称" required><Input value={name} maxLength={60} placeholder="例如：学校官网" disabled={tests.executing} onChange={(_, data) => setName(data.value)} /></Field>
          <Field label="网址" required><Input value={url} maxLength={1024} placeholder="https://example.com/" disabled={tests.executing} onChange={(_, data) => setUrl(data.value)} /></Field>
          {tests.error && <MessageBar intent="error"><MessageBarBody>{tests.error}</MessageBarBody></MessageBar>}
        </div></DialogContent>
        <DialogActions><Button disabled={tests.executing} onClick={() => { setAdding(false); tests.setError(''); }}>取消</Button>
          <Button appearance="primary" disabled={tests.executing || !name.trim() || !url.trim()} onClick={() => { void add(); }}>添加</Button></DialogActions>
      </DialogBody></DialogSurface>
    </Dialog>
  </>;
}
