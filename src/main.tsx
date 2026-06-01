import React from 'react';
import ReactDOM from 'react-dom/client';
import { ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import 'antd/dist/reset.css';
import 'ag-grid-community/styles/ag-grid.css';
import 'ag-grid-community/styles/ag-theme-quartz.css';
import './styles.css';
import App from './App';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ConfigProvider
      locale={zhCN}
      theme={{
        token: {
          colorPrimary: '#1677ff',
          borderRadius: 6,
          fontFamily: 'Microsoft YaHei, Segoe UI, Arial, sans-serif'
        },
        components: {
          Layout: {
            siderBg: '#101828',
            headerBg: '#ffffff',
            bodyBg: '#f4f7fb'
          }
        }
      }}
    >
      <App />
    </ConfigProvider>
  </React.StrictMode>
);
