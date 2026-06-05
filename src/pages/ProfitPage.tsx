import { App, Button, Card, DatePicker, Drawer, Select, Segmented, Space, Statistic, Table, Typography } from 'antd';
import type { EChartsOption } from 'echarts';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import EChart from '../components/EChart';
import PrintPreview from '../components/PrintPreview';
import { useAppStore } from '../store/appStore';
import { money, uniqueValues } from '../shared/format';
import type {
  OrderDetailDto,
  OrderDto,
  ProfitAnalyticsRequest,
  ProfitAnalyticsResponse,
  ProfitBreakdownDto,
  ProfitPeriod
} from '../shared/types';

const { RangePicker } = DatePicker;

type DateRange = [Dayjs, Dayjs];
type BreakdownMode = 'category' | 'customer';
type PieMetric = 'profit' | 'sales';

const periodOptions = [
  { label: '日', value: 'day' },
  { label: '月', value: 'month' },
  { label: '年', value: 'year' }
];

function defaultRange(period: ProfitPeriod): DateRange {
  if (period === 'month') {
    return [dayjs().startOf('year'), dayjs().endOf('year')];
  }
  if (period === 'year') {
    return [dayjs().subtract(4, 'year').startOf('year'), dayjs().endOf('year')];
  }
  return [dayjs().subtract(29, 'day'), dayjs()];
}

function requestDates(period: ProfitPeriod, range: DateRange) {
  if (period === 'month') {
    return {
      startDate: range[0].startOf('month').format('YYYY-MM-DD'),
      endDate: range[1].endOf('month').format('YYYY-MM-DD')
    };
  }
  if (period === 'year') {
    return {
      startDate: range[0].startOf('year').format('YYYY-MM-DD'),
      endDate: range[1].endOf('year').format('YYYY-MM-DD')
    };
  }
  return {
    startDate: range[0].format('YYYY-MM-DD'),
    endDate: range[1].format('YYYY-MM-DD')
  };
}

function compactMoney(value?: number) {
  return Number(value ?? 0).toFixed(2);
}

function amountTooltip(value: unknown) {
  return money(Number(value ?? 0));
}

function optionalMoney(value?: number) {
  return value === undefined ? '-' : money(value);
}

function optionalRate(value?: number) {
  return value === undefined ? '-' : `${value.toFixed(2)}%`;
}

function comparisonTitle(period: ProfitPeriod) {
  if (period === 'day') {
    return '环比上一日';
  }
  if (period === 'month') {
    return '同比去年同月';
  }
  return '同比上一年';
}

export default function ProfitPage() {
  const { message, modal } = App.useApp();
  const { products, customers, terms, features } = useAppStore();
  const [period, setPeriod] = useState<ProfitPeriod>('day');
  const [range, setRange] = useState<DateRange>(() => defaultRange('day'));
  const [customerId, setCustomerId] = useState<number>();
  const [category, setCategory] = useState<string>();
  const [analytics, setAnalytics] = useState<ProfitAnalyticsResponse>();
  const [rows, setRows] = useState<OrderDto[]>([]);
  const [detail, setDetail] = useState<OrderDetailDto>();
  const [loading, setLoading] = useState(false);
  const [breakdownMode, setBreakdownMode] = useState<BreakdownMode>('category');
  const [pieMetric, setPieMetric] = useState<PieMetric>('profit');

  const categories = useMemo(() => uniqueValues(products, (product) => product.category), [products]);
  const summary = analytics?.summary;
  const trendRows = analytics?.trend ?? [];
  const breakdownRows =
    breakdownMode === 'category' ? analytics?.categoryBreakdown ?? [] : analytics?.customerBreakdown ?? [];
  const pieRows = useMemo(() => {
    const valueOf = (row: ProfitBreakdownDto) =>
      pieMetric === 'profit' ? Math.max(row.profitAmount, 0) : Math.max(row.productSalesAmount, 0);
    return breakdownRows
      .map((row) => ({ name: row.name, value: valueOf(row) }))
      .filter((row) => row.value > 0);
  }, [breakdownRows, pieMetric]);

  async function load() {
    try {
      setLoading(true);
      const dates = requestDates(period, range);
      const request: ProfitAnalyticsRequest = {
        period,
        ...dates,
        customerId,
        category
      };
      const [nextAnalytics, nextRows] = await Promise.all([
        api.profitAnalytics(request),
        api.profitRecords({
          startDate: dates.startDate,
          endDate: dates.endDate,
          customerId,
          category
        })
      ]);
      setAnalytics(nextAnalytics);
      setRows(nextRows);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '利润数据加载失败');
    } finally {
      setLoading(false);
    }
  }

  async function preview(row: OrderDto) {
    try {
      setDetail(await api.order(row.id));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '订单详情加载失败');
    }
  }

  function voidOrder(row: OrderDto) {
    modal.confirm({
      title: `作废订单 ${row.orderNo}？`,
      content: `作废后会回滚库存流水${features.monthlyCredit ? `和${terms.credit}记录` : ''}。`,
      okText: '作废',
      okButtonProps: { danger: true },
      onOk: async () => {
        await api.voidOrder(row.id, { reason: '利润统计页作废' });
        message.success('订单已作废');
        await load();
      }
    });
  }

  function changePeriod(nextPeriod: ProfitPeriod) {
    setPeriod(nextPeriod);
    setRange(defaultRange(nextPeriod));
  }

  const trendOption = useMemo<EChartsOption>(
    () => ({
      color: ['#2563eb', '#0f766e', '#d97706', '#16a34a'],
      tooltip: { trigger: 'axis', valueFormatter: amountTooltip },
      legend: { top: 0 },
      grid: { top: 46, right: 24, bottom: 32, left: 62 },
      xAxis: { type: 'category', data: trendRows.map((row) => row.period), boundaryGap: false },
      yAxis: { type: 'value', axisLabel: { formatter: (value: number) => `${value}` } },
      series: [
        { name: '销售额', type: 'line', smooth: true, data: trendRows.map((row) => row.productSalesAmount) },
        { name: '实收', type: 'line', smooth: true, data: trendRows.map((row) => row.customerPayableAmount) },
        { name: '成本', type: 'line', smooth: true, data: trendRows.map((row) => row.costAmount) },
        {
          name: '利润',
          type: 'line',
          smooth: true,
          areaStyle: { opacity: 0.14 },
          data: trendRows.map((row) => row.profitAmount)
        }
      ]
    }),
    [trendRows]
  );

  const barOption = useMemo<EChartsOption>(
    () => ({
      color: ['#2563eb', '#d97706', '#16a34a', '#7c3aed'],
      tooltip: { trigger: 'axis' },
      legend: { top: 0 },
      grid: { top: 46, right: 48, bottom: 32, left: 62 },
      xAxis: { type: 'category', data: trendRows.map((row) => row.period) },
      yAxis: [
        { type: 'value', name: '金额' },
        { type: 'value', name: '单数' }
      ],
      series: [
        { name: '销售额', type: 'bar', data: trendRows.map((row) => row.productSalesAmount) },
        { name: '成本', type: 'bar', data: trendRows.map((row) => row.costAmount) },
        { name: '利润', type: 'bar', data: trendRows.map((row) => row.profitAmount) },
        { name: '订单数', type: 'bar', yAxisIndex: 1, data: trendRows.map((row) => row.orderCount) }
      ]
    }),
    [trendRows]
  );

  const pieOption = useMemo<EChartsOption>(
    () => ({
      color: ['#2563eb', '#16a34a', '#d97706', '#dc2626', '#7c3aed', '#0891b2', '#4d7c0f', '#be185d'],
      tooltip: {
        trigger: 'item',
        formatter: (params) => {
          const item = params as { name?: string; value?: number; percent?: number };
          return `${item.name ?? ''}<br />${pieMetric === 'profit' ? '利润' : '销售额'}：${money(
            item.value
          )}<br />占比：${item.percent ?? 0}%`;
        }
      },
      legend: { type: 'scroll', orient: 'vertical', right: 0, top: 16, bottom: 16 },
      series: [
        {
          name: pieMetric === 'profit' ? '利润占比' : '销售额占比',
          type: 'pie',
          radius: ['42%', '70%'],
          center: ['38%', '52%'],
          avoidLabelOverlap: true,
          label: { formatter: '{b}\n{d}%' },
          data: pieRows
        }
      ]
    }),
    [pieMetric, pieRows]
  );

  useEffect(() => {
    void load();
  }, []);

  return (
    <div className="page">
      <div className="page-title">
        <Typography.Title level={2}>利润统计</Typography.Title>
        <Space>
          <Button onClick={() => void load()} loading={loading}>
            查询
          </Button>
        </Space>
      </div>

      <Card>
        <div className="profit-filter-grid">
          <Segmented
            value={period}
            options={periodOptions}
            onChange={(value) => changePeriod(value as ProfitPeriod)}
          />
          <RangePicker
            value={range}
            picker={period === 'day' ? 'date' : period}
            allowClear={false}
            onChange={(value) => {
              if (value?.[0] && value[1]) {
                setRange([value[0], value[1]]);
              }
            }}
          />
          <Select
            allowClear
            showSearch
            placeholder={`全部${terms.customer}`}
            value={customerId}
            optionFilterProp="label"
            onChange={setCustomerId}
            options={customers.map((customer) => ({ value: customer.id, label: customer.name }))}
          />
          <Select
            allowClear
            showSearch
            placeholder={`全部${terms.category}`}
            value={category}
            optionFilterProp="label"
            onChange={setCategory}
            options={categories.map((item) => ({ value: item, label: item }))}
          />
        </div>
      </Card>

      <div className="stat-grid">
        <Card><Statistic title="出库单数" value={summary?.orderCount ?? 0} /></Card>
        <Card><Statistic title={`${terms.product}销售额`} value={compactMoney(summary?.productSalesAmount)} prefix="¥" /></Card>
        <Card><Statistic title={`${terms.customer}实收`} value={compactMoney(summary?.customerPayableAmount)} prefix="¥" /></Card>
        <Card><Statistic title="成本" value={compactMoney(summary?.costAmount)} prefix="¥" /></Card>
        <Card><Statistic title="利润" value={compactMoney(summary?.profitAmount)} prefix="¥" valueStyle={{ color: '#16a34a' }} /></Card>
      </div>

      <div className="profit-charts-grid">
        <Card
          title="利润趋势"
          className="chart-card chart-card-wide"
          extra={<Typography.Text type="secondary">销售额 / 实收 / 成本 / 利润</Typography.Text>}
        >
          <EChart option={trendOption} empty={trendRows.length === 0} />
        </Card>
        <Card title="周期对比" className="chart-card">
          <EChart option={barOption} empty={trendRows.length === 0} />
        </Card>
        <Card
          title={breakdownMode === 'category' ? `${terms.category}占比` : `${terms.customer}占比`}
          className="chart-card"
          extra={
            <Space>
              <Segmented
                size="small"
                value={breakdownMode}
                options={[
                  { label: terms.category, value: 'category' },
                  { label: terms.customer, value: 'customer' }
                ]}
                onChange={(value) => setBreakdownMode(value as BreakdownMode)}
              />
              <Segmented
                size="small"
                value={pieMetric}
                options={[
                  { label: '利润', value: 'profit' },
                  { label: '销售额', value: 'sales' }
                ]}
                onChange={(value) => setPieMetric(value as PieMetric)}
              />
            </Space>
          }
        >
          <EChart option={pieOption} empty={pieRows.length === 0} />
        </Card>
      </div>

      <Card title={`同比/环比分析：${comparisonTitle(period)}`}>
        <Table
          rowKey="period"
          size="small"
          pagination={false}
          loading={loading}
          dataSource={trendRows}
          scroll={{ x: 980 }}
          columns={[
            { title: '周期', dataIndex: 'period', fixed: 'left', width: 120 },
            { title: '对比周期', dataIndex: 'comparisonPeriod', width: 120 },
            { title: '销售额', render: (_, row) => money(row.productSalesAmount), align: 'right', width: 120 },
            { title: '对比销售额', render: (_, row) => optionalMoney(row.comparisonSalesAmount), align: 'right', width: 130 },
            { title: '销售额差值', render: (_, row) => optionalMoney(row.salesChangeAmount), align: 'right', width: 130 },
            { title: '销售额增长率', render: (_, row) => optionalRate(row.salesChangeRate), align: 'right', width: 130 },
            { title: '利润', render: (_, row) => money(row.profitAmount), align: 'right', width: 120 },
            { title: '对比利润', render: (_, row) => optionalMoney(row.comparisonProfitAmount), align: 'right', width: 120 },
            { title: '利润差值', render: (_, row) => optionalMoney(row.profitChangeAmount), align: 'right', width: 120 },
            { title: '利润增长率', render: (_, row) => optionalRate(row.profitChangeRate), align: 'right', width: 120 }
          ]}
        />
      </Card>

      <Table
        rowKey="id"
        loading={loading}
        dataSource={rows}
        columns={[
          { title: '日期', dataIndex: 'orderDate', width: 110 },
          { title: '单号', dataIndex: 'orderNo' },
          { title: terms.customer, dataIndex: 'customerName' },
          { title: '销售额', render: (_, row) => money(row.totals.productSalesAmount), align: 'right' },
          { title: '实收', render: (_, row) => money(row.totals.customerPayableAmount), align: 'right' },
          { title: '折现', render: (_, row) => money(row.totals.directDiscountAmount), align: 'right' },
          ...(features.monthlyCredit
            ? [{ title: `${terms.credit}抵扣`, render: (_: unknown, row: OrderDto) => money(row.totals.monthlyCreditUsed), align: 'right' as const }]
            : []),
          { title: '成本', render: (_, row) => money(row.totals.costAmount), align: 'right' },
          { title: '利润', render: (_, row) => money(row.totals.profitAmount), align: 'right' },
          {
            title: '操作',
            width: 150,
            render: (_, row) => (
              <Space>
                <Button size="small" onClick={() => void preview(row)}>预览</Button>
                <Button size="small" danger onClick={() => voidOrder(row)}>作废</Button>
              </Space>
            )
          }
        ]}
      />
      <Drawer
        title={detail ? `订单预览：${detail.order.orderNo}` : '订单预览'}
        open={!!detail}
        onClose={() => setDetail(undefined)}
        width={860}
      >
        <PrintPreview detail={detail} />
      </Drawer>
    </div>
  );
}
