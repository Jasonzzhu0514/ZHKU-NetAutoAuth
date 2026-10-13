import ReactDOM from 'react-dom/client';
import { FluentProvider } from '@fluentui/react-components';
import { desktopTheme } from './theme';
import { App } from './App';
import './styles.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <FluentProvider theme={desktopTheme}>
    <App />
  </FluentProvider>,
);
