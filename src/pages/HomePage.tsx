import {
  BarChartOutlined,
  EyeOutlined,
  InboxOutlined,
  ProductOutlined,
  ShoppingCartOutlined,
  TeamOutlined,
  TagsOutlined,
  WalletOutlined
} from '@ant-design/icons';
import { App, Button, Card, Col, Descriptions, Drawer, Row, Space, Statistic, Table, Typography } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api } from '../api/inventory';
import PrintPreview from '../components/PrintPreview';
import { money } from '../shared/format';
import type { DailyProfitSummary, InboundRecordDto, OrderDetailDto, OrderDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function HomePage() {
  const { message } = App.useApp();
  const [summary, setSummary] = useState<DailyProfitSummary>();
  const [orders, setOrders] = useState<OrderDto[]>([]);
  const [inbounds, setInbounds] = useState<InboundRecordDto[]>([]);
  const [orderDetail, setOrderDetail] = useState<OrderDetailDto>();
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const { products, setProductFilter } = useAppStore();
  const lowStockCount = products.filter((item) => item.currentStock <= item.safetyStock).length;

  useEffect(() => {
    async function load() {
      try {
        const date = dayjs().format('YYYY-MM-DD');
        const [profit, recentOrders, recentInbounds] = await Promise.all([
          api.dailyProfit(date),
          api.orders({ startDate: date, endDate: date, status: 'normal' }),
          api.inboundRecords({ startDate: date, endDate: date })
        ]);
        setSummary(profit);
        setOrders(recentOrders.slice(0, 5));
        setInbounds(recentInbounds.slice(0, 5));
      } catch (error) {
        message.warning(error instanceof Error ? error.message : '首页数据加载失败');
      }
    }
    void load();
  }, [message]);

  async function openOrderDetail(order: OrderDto) {
    setDetailOpen(true);
    setDetailLoading(true);
    setOrderDetail(undefined);
    try {
      setOrderDetail(await api.order(order.id));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '订单详情加载失败');
    } finally {
      setDetailLoading(false);
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>首页</Typography.Title>
          <Typography.Text type="secondary">高频入口与今日经营概览</Typography.Text>
        </div>
      </div>
      <div className="quick-grid">
        <Link to="/outbound"><Button className="quick-button" type="primary" block icon={<ShoppingCartOutlined />}>快速出库</Button></Link>
        <Link to="/inbound"><Button className="quick-button" block icon={<InboxOutlined />}>入库</Button></Link>
        <Link to="/products"><Button className="quick-button" block icon={<ProductOutlined />}>商品库存</Button></Link>
        <Link to="/rules"><Button className="quick-button" block icon={<TagsOutlined />}>客户规则</Button></Link>
        <Link to="/customers"><Button className="quick-button" block icon={<TeamOutlined />}>客户管理</Button></Link>
        <Link to="/suppliers"><Button className="quick-button" block icon={<TeamOutlined />}>供应商管理</Button></Link>
        <Link to="/credits"><Button className="quick-button" block icon={<WalletOutlined />}>月费账本</Button></Link>
        <Link to="/receivables"><Button className="quick-button" block icon={<WalletOutlined />}>欠款收款</Button></Link>
        <Link to="/profit"><Button className="quick-button" block icon={<BarChartOutlined />}>利润统计</Button></Link>
        <Link to="/inventory-report"><Button className="quick-button" block icon={<BarChartOutlined />}>进销存报表</Button></Link>
      </div>
      <div className="stat-grid">
        <Card><Statistic title="今日出库单数" value={summary?.orderCount ?? 0} /></Card>
        <Card><Statistic title="今日商品销售额" value={money(summary?.productSalesAmount)} /></Card>
        <Card><Statistic title="今日客户实收" value={money(summary?.customerPayableAmount)} /></Card>
        <Card><Statistic title="今日利润" value={money(summary?.profitAmount)} valueStyle={{ color: '#16a34a' }} /></Card>
        <Link to="/products" onClick={() => setProductFilter({ onlyLowStock: true })}>
          <Card className="clickable-card"><Statistic title="低库存商品" value={lowStockCount} valueStyle={{ color: lowStockCount > 0 ? '#d4380d' : '#16a34a' }} /></Card>
        </Link>
      </div>
      <Row gutter={14}>
        <Col span={12}>
          <Card title="最近出库单">
            <Table
              className="clickable-table"
              size="small"
              rowKey="id"
              dataSource={orders}
              pagination={false}
              onRow={(row) => ({
                role: 'button',
                tabIndex: 0,
                'aria-label': `查看出库单 ${row.orderNo} 详情`,
                onClick: () => void openOrderDetail(row),
                onKeyDown: (event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    void openOrderDetail(row);
                  }
                }
              })}
              columns={[
                { title: '单号', dataIndex: 'orderNo' },
                { title: '客户', dataIndex: 'customerName' },
                { title: '实收', render: (_, row) => money(row.totals.customerPayableAmount), align: 'right' },
                {
                  title: '操作',
                  width: 86,
                  align: 'center',
                  render: (_, row) => (
                    <Button
                      type="link"
                      size="small"
                      icon={<EyeOutlined />}
                      onClick={(event) => {
                        event.stopPropagation();
                        void openOrderDetail(row);
                      }}
                    >
                      详情
                    </Button>
                  )
                }
              ]}
            />
          </Card>
        </Col>
        <Col span={12}>
          <Card title="最近入库">
            <Table
              size="small"
              rowKey="id"
              dataSource={inbounds}
              pagination={false}
              columns={[
                { title: '商品', dataIndex: 'productName' },
                { title: '数量', dataIndex: 'quantity', align: 'right' },
                { title: '金额', render: (_, row) => money(row.amount), align: 'right' }
              ]}
            />
          </Card>
        </Col>
      </Row>
      <Drawer
        title={orderDetail ? `出库详情：${orderDetail.order.orderNo}` : '出库详情'}
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={760}
      >
        <Space style={{ marginBottom: 12 }}>
          <Button disabled={!orderDetail} onClick={() => orderDetail && void api.exportOrder(orderDetail.order.id).then((path) => message.success(`已导出：${path}`))}>重新导出</Button>
          <Button disabled={!orderDetail} onClick={() => orderDetail && void api.printOrderWithOptions(orderDetail.order.id).then((result) => message.success(result.message))}>打印</Button>
        </Space>
        <PrintPreview detail={orderDetail} />
        <Table
          loading={detailLoading}
          rowKey="id"
          dataSource={orderDetail?.items ?? []}
          pagination={false}
          size="small"
          title={() => orderDetail ? (
            <Space direction="vertical" size={12} style={{ width: '100%' }}>
              <Descriptions size="small" column={2} bordered>
                <Descriptions.Item label="客户">{orderDetail.order.customerName}</Descriptions.Item>
                <Descriptions.Item label="日期">{orderDetail.order.orderDate}</Descriptions.Item>
                <Descriptions.Item label="地址" span={2}>{orderDetail.order.customerAddress || '-'}</Descriptions.Item>
                <Descriptions.Item label="商品销售额">{money(orderDetail.order.totals.productSalesAmount)}</Descriptions.Item>
                <Descriptions.Item label="客户实收">{money(orderDetail.order.totals.customerPayableAmount)}</Descriptions.Item>
                <Descriptions.Item label="折现">{money(orderDetail.order.totals.directDiscountAmount)}</Descriptions.Item>
                <Descriptions.Item label="生成月费">{money(orderDetail.order.totals.brandSubsidyAmount)}</Descriptions.Item>
              </Descriptions>
              {orderDetail.order.remark ? <Typography.Text type="secondary">备注：{orderDetail.order.remark}</Typography.Text> : null}
            </Space>
          ) : null}
          columns={[
            { title: '商品', dataIndex: 'productName', ellipsis: true },
            { title: '类别', dataIndex: 'category', width: 110, ellipsis: true },
            { title: '数量', dataIndex: 'quantity', align: 'right', width: 90 },
            { title: '单价', render: (_, row) => money(row.unitPrice), align: 'right', width: 100 },
            { title: '金额', render: (_, row) => money(row.amount), align: 'right', width: 100 },
            { title: '备注', dataIndex: 'remark', ellipsis: true }
          ]}
        />
      </Drawer>
    </div>
  );
}
