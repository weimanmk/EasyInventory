import {
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
import type { MenuProps } from 'antd';
import { lazy, Suspense, type ComponentType, type ReactNode } from 'react';
import { Link, Route, Routes } from 'react-router-dom';
import type { FeatureFlagsDto, TermSettingsDto } from '../shared/types';

const HomePage = lazy(() => import('../pages/HomePage'));
const OutboundPage = lazy(() => import('../pages/OutboundPage'));
const InboundPage = lazy(() => import('../pages/InboundPage'));
const InventoryControlPage = lazy(() => import('../pages/InventoryControlPage'));
const ProductsPage = lazy(() => import('../pages/ProductsPage'));
const CustomersPage = lazy(() => import('../pages/CustomersPage'));
const SuppliersPage = lazy(() => import('../pages/SuppliersPage'));
const SupplierLedgerPage = lazy(() => import('../pages/SupplierLedgerPage'));
const RulesPage = lazy(() => import('../pages/RulesPage'));
const MonthlyCreditsPage = lazy(() => import('../pages/MonthlyCreditsPage'));
const ReceivablesPage = lazy(() => import('../pages/ReceivablesPage'));
const CustomerStatementPage = lazy(() => import('../pages/CustomerStatementPage'));
const ProfitPage = lazy(() => import('../pages/ProfitPage'));
const ProductRankingPage = lazy(() => import('../pages/ProductRankingPage'));
const CustomerAnalysisPage = lazy(() => import('../pages/CustomerAnalysisPage'));
const InventoryReportPage = lazy(() => import('../pages/InventoryReportPage'));
const DocumentsPage = lazy(() => import('../pages/DocumentsPage'));
const ExportPage = lazy(() => import('../pages/ExportPage'));
const SettingsPage = lazy(() => import('../pages/SettingsPage'));
const SetupPage = lazy(() => import('../pages/SetupPage'));

type AppRoute = {
  path: string;
  component: ComponentType;
};

export const appRoutes: AppRoute[] = [
  { path: '/', component: HomePage },
  { path: '/outbound', component: OutboundPage },
  { path: '/inbound', component: InboundPage },
  { path: '/inventory-control', component: InventoryControlPage },
  { path: '/products', component: ProductsPage },
  { path: '/customers', component: CustomersPage },
  { path: '/suppliers', component: SuppliersPage },
  { path: '/supplier-ledger', component: SupplierLedgerPage },
  { path: '/rules', component: RulesPage },
  { path: '/credits', component: MonthlyCreditsPage },
  { path: '/receivables', component: ReceivablesPage },
  { path: '/customer-statement', component: CustomerStatementPage },
  { path: '/profit', component: ProfitPage },
  { path: '/product-ranking', component: ProductRankingPage },
  { path: '/customer-analysis', component: CustomerAnalysisPage },
  { path: '/inventory-report', component: InventoryReportPage },
  { path: '/documents', component: DocumentsPage },
  { path: '/export', component: ExportPage },
  { path: '/settings', component: SettingsPage },
  { path: '/setup', component: SetupPage }
];

export function AppRoutes() {
  return (
    <Suspense fallback={<RouteFallback />}>
      <Routes>
        {appRoutes.map((route) => {
          const Page = route.component;
          return <Route key={route.path} path={route.path} element={<Page />} />;
        })}
      </Routes>
    </Suspense>
  );
}

function RouteFallback(): ReactNode {
  return <div className="route-fallback">加载中...</div>;
}

export function buildMenuItems(features: FeatureFlagsDto, terms: TermSettingsDto): MenuProps['items'] {
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
}
