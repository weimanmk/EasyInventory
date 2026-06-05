import {
  AppstoreOutlined,
  BarChartOutlined,
  DatabaseOutlined,
  FileExcelOutlined,
  FileTextOutlined,
  HomeOutlined,
  InboxOutlined,
  ProductOutlined,
  ReconciliationOutlined,
  SettingOutlined,
  ShoppingCartOutlined,
  TeamOutlined,
  TagsOutlined,
  WalletOutlined
} from '@ant-design/icons';
import { App as AntApp, Layout, Menu, Space, Tag, Typography, message } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo } from 'react';
import { HashRouter, Link, Route, Routes, useLocation, useNavigate } from 'react-router-dom';
import { api } from './api/inventory';
import { useAppStore } from './store/appStore';
import HomePage from './pages/HomePage';
import OutboundPage from './pages/OutboundPage';
import InboundPage from './pages/InboundPage';
import InventoryControlPage from './pages/InventoryControlPage';
import ProductsPage from './pages/ProductsPage';
import CustomersPage from './pages/CustomersPage';
import SuppliersPage from './pages/SuppliersPage';
import SupplierLedgerPage from './pages/SupplierLedgerPage';
import RulesPage from './pages/RulesPage';
import MonthlyCreditsPage from './pages/MonthlyCreditsPage';
import ReceivablesPage from './pages/ReceivablesPage';
import CustomerStatementPage from './pages/CustomerStatementPage';
import ProfitPage from './pages/ProfitPage';
import InventoryReportPage from './pages/InventoryReportPage';
import ProductRankingPage from './pages/ProductRankingPage';
import CustomerAnalysisPage from './pages/CustomerAnalysisPage';
import DocumentsPage from './pages/DocumentsPage';
import SettingsPage from './pages/SettingsPage';
import ExportPage from './pages/ExportPage';
import SetupPage from './pages/SetupPage';

const { Header, Sider, Content, Footer } = Layout;

function Shell() {
  const location = useLocation();
  const navigate = useNavigate();
  const {
    status,
    setupStatus,
    merchant,
    terms,
    features,
    setStatus,
    setSetupStatus,
    setMerchant,
    setTerms,
    setFeatures,
    setProducts,
    setCustomers
  } = useAppStore();

  useEffect(() => {
    async function boot() {
      try {
        const [appStatus, products, customers, nextSetupStatus, nextMerchant, nextTerms, nextFeatures] = await Promise.all([
          api.status(),
          api.products({ isActive: true }),
          api.customers({ isActive: true }),
          api.setupStatus(),
          api.merchantProfile(),
          api.termSettings(),
          api.featureFlags()
        ]);
        setStatus(appStatus);
        setProducts(products);
        setCustomers(customers);
        setSetupStatus(nextSetupStatus);
        setMerchant(nextMerchant);
        setTerms(nextTerms);
        setFeatures(nextFeatures);
      } catch (error) {
        message.error(error instanceof Error ? error.message : '初始化失败');
      }
    }
    void boot();
  }, [setCustomers, setFeatures, setMerchant, setProducts, setSetupStatus, setStatus, setTerms]);

  useEffect(() => {
    if (setupStatus?.completed === false && location.pathname !== '/setup') {
      navigate('/setup', { replace: true });
    }
  }, [location.pathname, navigate, setupStatus?.completed]);

  const menuItems = useMemo(() => {
    const allItems = [
      { key: '/', icon: <HomeOutlined />, label: <Link to="/">首页</Link> },
      { key: '/outbound', icon: <ShoppingCartOutlined />, label: <Link to="/outbound">快速出库</Link> },
      { key: '/inbound', icon: <InboxOutlined />, label: <Link to="/inbound">入库</Link> },
      features.inventoryControl
        ? { key: '/inventory-control', icon: <ReconciliationOutlined />, label: <Link to="/inventory-control">库存盘点</Link> }
        : null,
      { key: '/products', icon: <ProductOutlined />, label: <Link to="/products">{terms.product}库存</Link> },
      { key: '/customers', icon: <TeamOutlined />, label: <Link to="/customers">{terms.customer}管理</Link> },
      { key: '/suppliers', icon: <DatabaseOutlined />, label: <Link to="/suppliers">供应商管理</Link> },
      features.supplierLedger
        ? { key: '/supplier-ledger', icon: <BarChartOutlined />, label: <Link to="/supplier-ledger">供应商采购台账</Link> }
        : null,
      features.customerRules
        ? { key: '/rules', icon: <TagsOutlined />, label: <Link to="/rules">{terms.rule}</Link> }
        : null,
      features.monthlyCredit
        ? { key: '/credits', icon: <WalletOutlined />, label: <Link to="/credits">{terms.credit}账本</Link> }
        : null,
      features.receivables
        ? { key: '/receivables', icon: <WalletOutlined />, label: <Link to="/receivables">欠款收款</Link> }
        : null,
      { key: '/customer-statement', icon: <ReconciliationOutlined />, label: <Link to="/customer-statement">{terms.customer}对账单</Link> },
      { key: '/profit', icon: <BarChartOutlined />, label: <Link to="/profit">利润统计</Link> },
      features.productRanking
        ? { key: '/product-ranking', icon: <BarChartOutlined />, label: <Link to="/product-ranking">{terms.product}经营排行</Link> }
        : null,
      features.customerAnalysis
        ? { key: '/customer-analysis', icon: <BarChartOutlined />, label: <Link to="/customer-analysis">{terms.customer}经营分析</Link> }
        : null,
      { key: '/inventory-report', icon: <BarChartOutlined />, label: <Link to="/inventory-report">进销存报表</Link> },
      { key: '/documents', icon: <FileTextOutlined />, label: <Link to="/documents">单据档案</Link> },
      { key: '/export', icon: <FileExcelOutlined />, label: <Link to="/export">数据导出</Link> },
      { key: '/settings', icon: <SettingOutlined />, label: <Link to="/settings">系统设置</Link> }
    ];
    return allItems.filter((item): item is NonNullable<typeof item> => item !== null);
  }, [features, terms]);

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
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/outbound" element={<OutboundPage />} />
            <Route path="/inbound" element={<InboundPage />} />
            <Route path="/inventory-control" element={<InventoryControlPage />} />
            <Route path="/products" element={<ProductsPage />} />
            <Route path="/customers" element={<CustomersPage />} />
            <Route path="/suppliers" element={<SuppliersPage />} />
            <Route path="/supplier-ledger" element={<SupplierLedgerPage />} />
            <Route path="/rules" element={<RulesPage />} />
            <Route path="/credits" element={<MonthlyCreditsPage />} />
            <Route path="/receivables" element={<ReceivablesPage />} />
            <Route path="/customer-statement" element={<CustomerStatementPage />} />
            <Route path="/profit" element={<ProfitPage />} />
            <Route path="/product-ranking" element={<ProductRankingPage />} />
            <Route path="/customer-analysis" element={<CustomerAnalysisPage />} />
            <Route path="/inventory-report" element={<InventoryReportPage />} />
            <Route path="/documents" element={<DocumentsPage />} />
            <Route path="/export" element={<ExportPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/setup" element={<SetupPage />} />
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
