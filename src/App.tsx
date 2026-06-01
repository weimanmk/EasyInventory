import {
  AppstoreOutlined,
  BarChartOutlined,
  DatabaseOutlined,
  FileExcelOutlined,
  FileTextOutlined,
  HomeOutlined,
  InboxOutlined,
  ProductOutlined,
  SettingOutlined,
  ShoppingCartOutlined,
  TeamOutlined,
  TagsOutlined,
  WalletOutlined
} from '@ant-design/icons';
import { App as AntApp, Layout, Menu, Space, Tag, Typography, message } from 'antd';
import dayjs from 'dayjs';
import { useEffect } from 'react';
import { HashRouter, Link, Route, Routes, useLocation } from 'react-router-dom';
import { api } from './api/inventory';
import { useAppStore } from './store/appStore';
import HomePage from './pages/HomePage';
import OutboundPage from './pages/OutboundPage';
import InboundPage from './pages/InboundPage';
import ProductsPage from './pages/ProductsPage';
import CustomersPage from './pages/CustomersPage';
import SuppliersPage from './pages/SuppliersPage';
import RulesPage from './pages/RulesPage';
import MonthlyCreditsPage from './pages/MonthlyCreditsPage';
import ReceivablesPage from './pages/ReceivablesPage';
import ProfitPage from './pages/ProfitPage';
import InventoryReportPage from './pages/InventoryReportPage';
import DocumentsPage from './pages/DocumentsPage';
import SettingsPage from './pages/SettingsPage';
import ExportPage from './pages/ExportPage';

const { Header, Sider, Content, Footer } = Layout;

const menuItems = [
  { key: '/', icon: <HomeOutlined />, label: <Link to="/">首页</Link> },
  { key: '/outbound', icon: <ShoppingCartOutlined />, label: <Link to="/outbound">快速出库</Link> },
  { key: '/inbound', icon: <InboxOutlined />, label: <Link to="/inbound">入库</Link> },
  { key: '/products', icon: <ProductOutlined />, label: <Link to="/products">商品库存</Link> },
  { key: '/customers', icon: <TeamOutlined />, label: <Link to="/customers">客户管理</Link> },
  { key: '/suppliers', icon: <DatabaseOutlined />, label: <Link to="/suppliers">供应商管理</Link> },
  { key: '/rules', icon: <TagsOutlined />, label: <Link to="/rules">客户商品规则</Link> },
  { key: '/credits', icon: <WalletOutlined />, label: <Link to="/credits">月费账本</Link> },
  { key: '/receivables', icon: <WalletOutlined />, label: <Link to="/receivables">欠款收款</Link> },
  { key: '/profit', icon: <BarChartOutlined />, label: <Link to="/profit">利润统计</Link> },
  { key: '/inventory-report', icon: <BarChartOutlined />, label: <Link to="/inventory-report">进销存报表</Link> },
  { key: '/documents', icon: <FileTextOutlined />, label: <Link to="/documents">单据档案</Link> },
  { key: '/export', icon: <FileExcelOutlined />, label: <Link to="/export">数据导出</Link> },
  { key: '/settings', icon: <SettingOutlined />, label: <Link to="/settings">系统设置</Link> }
];

function Shell() {
  const location = useLocation();
  const { status, setStatus, setProducts, setCustomers } = useAppStore();

  useEffect(() => {
    async function boot() {
      try {
        const [appStatus, products, customers] = await Promise.all([
          api.status(),
          api.products({ isActive: true }),
          api.customers({ isActive: true })
        ]);
        setStatus(appStatus);
        setProducts(products);
        setCustomers(customers);
      } catch (error) {
        message.error(error instanceof Error ? error.message : '初始化失败');
      }
    }
    void boot();
  }, [setCustomers, setProducts, setStatus]);

  return (
    <Layout className="app-shell">
      <Sider width={224} className="app-sider">
        <div className="brand">
          <AppstoreOutlined />
          <div>
            <strong>EasyInventory</strong>
            <span>本地库存计价打单</span>
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
            <Typography.Title level={4}>本地库存计价打单系统</Typography.Title>
            <Tag color="blue">V{status?.version ?? '1.0.0'}</Tag>
          </Space>
          <Space size={12}>
            <Tag icon={<DatabaseOutlined />} color="green">SQLite 本地</Tag>
            <Tag icon={<FileExcelOutlined />} color="cyan">{dayjs().format('YYYY-MM-DD')}</Tag>
          </Space>
        </Header>
        <Content className="app-content">
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/outbound" element={<OutboundPage />} />
            <Route path="/inbound" element={<InboundPage />} />
            <Route path="/products" element={<ProductsPage />} />
            <Route path="/customers" element={<CustomersPage />} />
            <Route path="/suppliers" element={<SuppliersPage />} />
            <Route path="/rules" element={<RulesPage />} />
            <Route path="/credits" element={<MonthlyCreditsPage />} />
            <Route path="/receivables" element={<ReceivablesPage />} />
            <Route path="/profit" element={<ProfitPage />} />
            <Route path="/inventory-report" element={<InventoryReportPage />} />
            <Route path="/documents" element={<DocumentsPage />} />
            <Route path="/export" element={<ExportPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </Content>
        <Footer className="app-footer">
          <span>数据库：{status?.databasePath ?? '初始化中'}</span>
          <span>订单目录：{status?.ordersDir ?? '-'}</span>
        </Footer>
      </Layout>
    </Layout>
  );
}

export default function App() {
  return (
    <HashRouter>
      <AntApp>
        <Shell />
      </AntApp>
    </HashRouter>
  );
}
