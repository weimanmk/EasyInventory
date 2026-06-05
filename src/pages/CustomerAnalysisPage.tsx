import { Alert, App, Button, Card, DatePicker, InputNumber, Select, Segmented, Space, Statistic, Table, Typography } from 'antd';
import type { EChartsOption } from 'echarts';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import EChart from '../components/EChart';
import { money, uniqueValues } from '../shared/format';
import type { CustomerAnalysisRankBy, CustomerAnalysisRowDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

const { RangePicker } = DatePicker;

type DateRange = [Dayjs, Dayjs];
type AnalysisPeriod = 'month' | 'year' | 'custom';

const periodOptions = [
  { label: '月', value: 'month' },
  { label: '年', value: 'year' },
  { label: '自定义', value: 'custom' }
];

const rankOptions: Array<{ label: string; value: CustomerAnalysisRankBy }> = [
  { label: '销售额', value: 'sales_amount' },
  { label: '利润', value: 'profit_amount' },
  { label: '欠款', value: 'balance_amount' }
];

function defaultRange(period: AnalysisPeriod): DateRange {
  if (period === 'year') {
    return [dayjs().startOf('year'), dayjs().endOf('year')];
  }
  return [dayjs().startOf('month'), dayjs().endOf('month')];
}

function requestRange(period: AnalysisPeriod, range: DateRange) {
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

function rankLabel(rankBy: CustomerAnalysisRankBy) {
  return rankOptions.find((item) => item.value === rankBy)?.label ?? '利润';
}

function rankValue(row: CustomerAnalysisRowDto, rankBy: CustomerAnalysisRankBy) {
  if (rankBy === 'sales_amount') {
    return row.salesAmount;
  }
  if (rankBy === 'balance_amount') {
    return row.balanceAmount;
  }
  return row.profitAmount;
}

export default function CustomerAnalysisPage() {
  const { message } = App.useApp();
  const { products, terms, features } = useAppStore();
  const [period, setPeriod] = useState<AnalysisPeriod>('month');
  const [range, setRange] = useState<DateRange>(() => defaultRange('month'));
  const [category, setCategory] = useState<string>();
  const [rankBy, setRankBy] = useState<CustomerAnalysisRankBy>('sales_amount');
  const [limit, setLimit] = useState(20);
  const [rows, setRows] = useState<CustomerAnalysisRowDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const categories = useMemo(() => uniqueValues(products, (product) => product.category), [products]);

  const totalSales = rows.reduce((sum, row) => sum + row.salesAmount, 0);
  const totalProfit = rows.reduce((sum, row) => sum + row.profitAmount, 0);
  const totalBalance = rows.reduce((sum, row) => sum + Math.max(row.balanceAmount, 0), 0);
  const activeCustomerCount = rows.filter((row) => row.orderCount > 0).length;
  const repurchaseRows = rows.filter((row) => row.averageRepurchaseDays !== undefined);
  const averageRepurchase =
    repurchaseRows.length > 0
      ? repurchaseRows.reduce((sum, row) => sum + Number(row.averageRepurchaseDays ?? 0), 0) / repurchaseRows.length
      : undefined;

  async function loadRows() {
    if (!features.customerAnalysis) {
      setRows([]);
      return;
    }
    const dates = requestRange(period, range);
    setLoading(true);
    try {
      const result = await api.customerAnalysis({
        ...dates,
        category,
        rankBy,
        limit
      });
      setRows(result.rows);
    } catch (error) {
      message.error(error instanceof Error ? error.message : `${terms.customer}经营分析加载失败`);
    } finally {
      setLoading(false);
    }
  }

  async function exportAnalysis(openAfter = false) {
    const dates = requestRange(period, range);
    setExporting(true);
    try {
      const path = await api.exportData({
        exportType: 'customer_analysis',
        ...dates,
        category,
        rankBy
      });
      message.success(`已导出：${path}`);
      if (openAfter) {
        await api.openExportsFolder();
      }
    } catch (error) {
      message.error(error instanceof Error ? error.message : `导出${terms.customer}经营分析失败`);
    } finally {
      setExporting(false);
    }
  }

  function changePeriod(nextPeriod: AnalysisPeriod) {
    setPeriod(nextPeriod);
    if (nextPeriod !== 'custom') {
      setRange(defaultRange(nextPeriod));
    }
  }

  const rankBarOption = useMemo<EChartsOption>(() => ({
    color: ['#0f766e'],
    tooltip: { trigger: 'axis', valueFormatter: (value) => money(Number(value ?? 0)) },
    grid: { left: 62, right: 24, top: 28, bottom: 78 },
    xAxis: {
      type: 'category',
      data: rows.map((row) => row.customerName),
      axisLabel: { interval: 0, rotate: 28 }
    },
    yAxis: { type: 'value', name: rankLabel(rankBy) },
    series: [
      {
        name: rankLabel(rankBy),
        type: 'bar',
        data: rows.map((row) => rankValue(row, rankBy))
      }
    ]
  }), [rankBy, rows]);

  const balancePieOption = useMemo<EChartsOption>(() => {
    const data = rows
      .filter((row) => row.balanceAmount > 0)
      .map((row) => ({ name: row.customerName, value: row.balanceAmount }));
    return {
      color: ['#dc2626', '#d97706', '#7c3aed', '#2563eb', '#16a34a', '#0891b2', '#be185d'],
      tooltip: {
        trigger: 'item',
        formatter: (params) => {
          const item = params as { name?: string; value?: number; percent?: number };
          return `${item.name ?? ''}<br />欠款：${money(item.value)}<br />占比：${item.percent ?? 0}%`;
        }
      },
      legend: { type: 'scroll', orient: 'vertical', right: 0, top: 10, bottom: 10 },
      series: [
        {
          name: '欠款占比',
          type: 'pie',
          radius: ['42%', '70%'],
          center: ['38%', '52%'],
          label: { formatter: '{b}\n{d}%' },
          data
        }
      ]
    };
  }, [rows]);

  useEffect(() => {
    void loadRows();
  }, [features.customerAnalysis]);

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>{terms.customer}经营分析</Typography.Title>
          <Typography.Text type="secondary">
            {terms.customer}销售、利润、欠款、复购和偏好{terms.product}分析
          </Typography.Text>
        </div>
        <Space>
          <Button loading={exporting} disabled={!features.customerAnalysis} onClick={() => void exportAnalysis(false)}>导出</Button>
          <Button type="primary" loading={loading} disabled={!features.customerAnalysis} onClick={() => void loadRows()}>查询</Button>
        </Space>
      </div>

      {!features.customerAnalysis && (
        <Alert
          type="info"
          showIcon
          message={`${terms.customer}经营分析功能已关闭`}
          description={`可以在系统设置的功能开关中重新开启，历史${terms.customer}交易数据会保留。`}
        />
      )}
      {features.customerAnalysis && (
        <>
          <div className="toolbar panel">
            <Segmented
              options={periodOptions}
              value={period}
              onChange={(value) => changePeriod(value as AnalysisPeriod)}
            />
            <RangePicker
              value={range}
              onChange={(values) => {
                if (values?.[0] && values[1]) {
                  setRange([values[0], values[1]]);
                  setPeriod('custom');
                }
              }}
            />
            <Select
              allowClear
              placeholder={terms.category}
              value={category}
              style={{ width: 180 }}
              options={categories.map((item) => ({ value: item, label: item }))}
              onChange={setCategory}
            />
            <Select
              value={rankBy}
              style={{ width: 150 }}
              options={rankOptions}
              onChange={setRankBy}
            />
            <InputNumber
              min={1}
              max={100}
              value={limit}
              addonBefore="Top"
              style={{ width: 128 }}
              onChange={(value) => setLimit(Number(value ?? 20))}
            />
            <Button loading={exporting} onClick={() => void exportAnalysis(true)}>导出并打开目录</Button>
          </div>

          <div className="stat-grid">
            <Card><Statistic title="销售额" value={money(totalSales)} /></Card>
            <Card><Statistic title="利润" value={money(totalProfit)} /></Card>
            <Card><Statistic title="当前欠款" value={money(totalBalance)} /></Card>
            <Card><Statistic title={`有交易${terms.customer}`} value={activeCustomerCount} /></Card>
            <Card><Statistic title="平均复购间隔" value={averageRepurchase === undefined ? '-' : `${averageRepurchase.toFixed(1)} 天`} /></Card>
          </div>

          <div className="two-column-grid">
            <Card title={`${rankLabel(rankBy)}排行`}>
              <EChart option={rankBarOption} height={340} empty={rows.length === 0} />
            </Card>
            <Card title="欠款占比">
              <EChart
                option={balancePieOption}
                height={340}
                empty={rows.filter((row) => row.balanceAmount > 0).length === 0}
              />
            </Card>
          </div>

          <Card title={`${terms.customer}分析明细`}>
            <Table
              rowKey="customerId"
              size="small"
              loading={loading}
              dataSource={rows}
              scroll={{ x: 1280 }}
              columns={[
                { title: terms.customer, dataIndex: 'customerName', fixed: 'left', width: 200 },
                { title: terms.region, dataIndex: 'region', width: 110 },
                { title: '订单数', dataIndex: 'orderCount', align: 'right', width: 100 },
                { title: '销售额', render: (_, row) => money(row.salesAmount), align: 'right', width: 120 },
                { title: '成本', render: (_, row) => money(row.costAmount), align: 'right', width: 120 },
                { title: '利润', render: (_, row) => money(row.profitAmount), align: 'right', width: 120 },
                { title: '当前欠款', render: (_, row) => money(row.balanceAmount), align: 'right', width: 130 },
                { title: '最近购买', dataIndex: 'recentOrderDate', width: 120 },
                {
                  title: '平均复购间隔',
                  render: (_, row) =>
                    row.averageRepurchaseDays === undefined ? '-' : `${row.averageRepurchaseDays.toFixed(1)} 天`,
                  align: 'right',
                  width: 140
                },
                { title: `偏好${terms.product}`, dataIndex: 'favoriteProducts', width: 260 }
              ]}
            />
          </Card>
        </>
      )}
    </div>
  );
}
