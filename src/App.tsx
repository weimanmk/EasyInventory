import { App as AntApp } from 'antd';
import { HashRouter } from 'react-router-dom';
import { AppShell } from './app/AppShell';

export default function App() {
  return (
    <HashRouter>
      <AntApp>
        <AppShell />
      </AntApp>
    </HashRouter>
  );
}
