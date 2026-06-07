import { AppstoreOutlined, DatabaseOutlined, FileExcelOutlined } from '@ant-design/icons';
import { Layout, Menu, Space, Tag, Typography } from 'antd';
import dayjs from 'dayjs';
import { useMemo } from 'react';
import { useLocation } from 'react-router-dom';
import { useAppStore } from '../store/appStore';
import { AppRoutes, buildMenuItems } from './routes';
import { useAppBootstrap } from './useAppBootstrap';

const { Header, Sider, Content, Footer } = Layout;

export function AppShell() {
  useAppBootstrap();

  const location = useLocation();
  const { status, merchant, terms, features } = useAppStore();
  const menuItems = useMemo(() => buildMenuItems(features, terms), [features, terms]);

  return (
    <Layout className="app-shell">
      <Sider width={224} className="app-sider">
        <div className="brand">
          <AppstoreOutlined />
          <div>
            <strong>EasyInventory</strong>
            <span>{merchant.name || '我的商行'}</span>
          </div>
        </div>
        <Menu
          theme="dark"
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          className="nav-menu"
        />
      </Sider>
      <Layout>
        <Header className="app-header">
          <Space size={16}>
            <Typography.Title level={4}>{merchant.name || '我的商行'} · 本地库存计价打单系统</Typography.Title>
            <Tag color="blue">V{status?.version ?? '1.0.0'}</Tag>
          </Space>
          <Space size={12}>
            <Tag icon={<DatabaseOutlined />} color="green">SQLite 本地</Tag>
            <Tag icon={<FileExcelOutlined />} color="cyan">{dayjs().format('YYYY-MM-DD')}</Tag>
          </Space>
        </Header>
        <Content className="app-content">
          <AppRoutes />
        </Content>
        <Footer className="app-footer">
          <span>数据库：{status?.databasePath ?? '初始化中'}</span>
          <span>订单目录：{status?.ordersDir ?? '-'}</span>
        </Footer>
      </Layout>
    </Layout>
  );
}
