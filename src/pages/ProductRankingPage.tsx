import { Alert, App, Button, Card, DatePicker, InputNumber, Select, Segmented, Space, Statistic, Table, Typography } from 'antd';
import type { EChartsOption } from 'echarts';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import EChart from '../components/EChart';
import { money, qty, uniqueValues } from '../shared/format';
import type { ProductRankingRankBy, ProductRankingRowDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

const { RangePicker } = DatePicker;

type DateRange = [Dayjs, Dayjs];
type RankingPeriod = 'day' | 'month' | 'year' | 'custom';

const periodOptions = [
  { label: '日', value: 'day' },
  { label: '月', value: 'month' },
  { label: '年', value: 'year' },
  { label: '自定义', value: 'custom' }
];

const rankingOptions: Array<{ label: string; value: ProductRankingRankBy }> = [
  { label: '销量', value: 'sales_quantity' },
  { label: '销售额', value: 'sales_amount' },
  { label: '利润', value: 'profit_amount' },
  { label: '赠品成本', value: 'gift_cost_amount' }
];

function defaultRange(period: RankingPeriod): DateRange {
  if (period === 'day') {
    return [dayjs(), dayjs()];
  }
  if (period === 'year') {
    return [dayjs().startOf('year'), dayjs().endOf('year')];
  }
  return [dayjs().startOf('month'), dayjs().endOf('month')];
}

function requestRange(period: RankingPeriod, range: DateRange) {
  if (period === 'day') {
    return {
      startDate: range[0].format('YYYY-MM-DD'),
      endDate: range[0].format('YYYY-MM-DD')
    };
  }
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

function rankLabel(rankBy: ProductRankingRankBy) {
  return rankingOptions.find((item) => item.value === rankBy)?.label ?? '利润';
}

function rankValue(row: ProductRankingRowDto, rankBy: ProductRankingRankBy) {
  if (rankBy === 'sales_quantity') {
    return row.salesQuantity;
  }
  if (rankBy === 'sales_amount') {
    return row.salesAmount;
  }
  if (rankBy === 'gift_cost_amount') {
    return row.giftCostAmount;
  }
  return row.profitAmount;
}

export default function ProductRankingPage() {
  const { message } = App.useApp();
  const { products, terms, features } = useAppStore();
  const [period, setPeriod] = useState<RankingPeriod>('month');
  const [range, setRange] = useState<DateRange>(() => defaultRange('month'));
  const [category, setCategory] = useState<string>();
  const [rankBy, setRankBy] = useState<ProductRankingRankBy>('profit_amount');
  const [limit, setLimit] = useState(20);
  const [rows, setRows] = useState<ProductRankingRowDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const categories = useMemo(() => uniqueValues(products, (product) => product.category), [products]);

  const totalSalesAmount = rows.reduce((sum, row) => sum + row.salesAmount, 0);
  const totalProfitAmount = rows.reduce((sum, row) => sum + row.profitAmount, 0);
  const totalSalesQuantity = rows.reduce((sum, row) => sum + row.salesQuantity, 0);
  const totalGiftCostAmount = rows.reduce((sum, row) => sum + row.giftCostAmount, 0);

  async function loadRows() {
    if (!features.productRanking) {
      setRows([]);
      return;
    }
    const dates = requestRange(period, range);
    setLoading(true);
    try {
      setRows(await api.productRanking({
        ...dates,
        category,
        rankBy,
        limit
      }));
    } catch (error) {
      message.error(error instanceof Error ? error.message : `${terms.product}经营排行加载失败`);
    } finally {
      setLoading(false);
    }
  }

  async function exportRanking(openAfter = false) {
    const dates = requestRange(period, range);
    setExporting(true);
    try {
      const path = await api.exportData({
        exportType: 'product_ranking',
        ...dates,
        category,
        rankBy
      });
      message.success(`已导出：${path}`);
      if (openAfter) {
        await api.openExportsFolder();
      }
    } catch (error) {
      message.error(error instanceof Error ? error.message : `导出${terms.product}经营排行失败`);
    } finally {
      setExporting(false);
    }
  }

  function changePeriod(nextPeriod: RankingPeriod) {
    setPeriod(nextPeriod);
    if (nextPeriod !== 'custom') {
      setRange(defaultRange(nextPeriod));
    }
  }

  const barOption = useMemo<EChartsOption>(() => ({
    color: ['#2563eb'],
    tooltip: {
      trigger: 'axis',
      valueFormatter: (value) =>
        rankBy === 'sales_quantity' ? qty(Number(value ?? 0)) : money(Number(value ?? 0))
    },
    grid: { left: 62, right: 24, top: 28, bottom: 74 },
    xAxis: {
      type: 'category',
      data: rows.map((row) => row.productName),
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

  const profitPieOption = useMemo<EChartsOption>(() => {
    const positiveRows = rows
      .map((row) => ({ name: row.productName, value: Math.max(row.profitAmount, 0) }))
      .filter((row) => row.value > 0);
    return {
      color: ['#2563eb', '#16a34a', '#d97706', '#7c3aed', '#0891b2', '#be185d', '#4d7c0f'],
      tooltip: {
        trigger: 'item',
        formatter: (params) => {
          const item = params as { name?: string; value?: number; percent?: number };
          return `${item.name ?? ''}<br />利润：${money(item.value)}<br />占比：${item.percent ?? 0}%`;
        }
      },
      legend: { type: 'scroll', orient: 'vertical', right: 0, top: 10, bottom: 10 },
      series: [
        {
          name: '利润占比',
          type: 'pie',
          radius: ['42%', '70%'],
          center: ['38%', '52%'],
          label: { formatter: '{b}\n{d}%' },
          data: positiveRows
        }
      ]
    };
  }, [rows]);

  useEffect(() => {
    void loadRows();
  }, [features.productRanking]);

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>{terms.product}经营排行</Typography.Title>
          <Typography.Text type="secondary">按{terms.product}明细统计销量、销售额、利润和赠品成本</Typography.Text>
        </div>
        <Space>
          <Button loading={exporting} disabled={!features.productRanking} onClick={() => void exportRanking(false)}>导出</Button>
          <Button type="primary" loading={loading} disabled={!features.productRanking} onClick={() => void loadRows()}>查询</Button>
        </Space>
      </div>

      {!features.productRanking && (
        <Alert
          type="info"
          showIcon
          message={`${terms.product}经营排行功能已关闭`}
          description={`可以在系统设置的功能开关中重新开启，历史${terms.product}销售数据会保留。`}
        />
      )}
      {features.productRanking && (
        <>
          <div className="toolbar panel">
            <Segmented
              options={periodOptions}
              value={period}
              onChange={(value) => changePeriod(value as RankingPeriod)}
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
              options={rankingOptions}
              onChange={setRankBy}
            />
            <InputNumber
              min={1}
              max={100}
              value={limit}
              controls
              addonBefore="Top"
              style={{ width: 128 }}
              onChange={(value) => setLimit(Number(value ?? 20))}
            />
            <Button loading={exporting} onClick={() => void exportRanking(true)}>导出并打开目录</Button>
          </div>

          <div className="stat-grid">
            <Card><Statistic title="销售额" value={money(totalSalesAmount)} /></Card>
            <Card><Statistic title="利润" value={money(totalProfitAmount)} /></Card>
            <Card><Statistic title="销量" value={qty(totalSalesQuantity)} /></Card>
            <Card><Statistic title="赠品成本" value={money(totalGiftCostAmount)} /></Card>
            <Card><Statistic title={`${terms.product}数`} value={rows.length} /></Card>
          </div>

          <div className="two-column-grid">
            <Card title={`${rankLabel(rankBy)}排行`}>
              <EChart option={barOption} height={340} empty={rows.length === 0} />
            </Card>
            <Card title="利润占比">
              <EChart
                option={profitPieOption}
                height={340}
                empty={rows.filter((row) => row.profitAmount > 0).length === 0}
              />
            </Card>
          </div>

          <Card title={`${terms.product}排行明细`}>
            <Table
              rowKey={(row) => `${row.productId}-${row.productName}`}
              size="small"
              loading={loading}
              dataSource={rows}
              scroll={{ x: 1180 }}
              columns={[
                { title: terms.product, dataIndex: 'productName', fixed: 'left', width: 220 },
                { title: terms.category, dataIndex: 'category', width: 120 },
                { title: '订单数', dataIndex: 'orderCount', align: 'right', width: 100 },
                { title: '销量', render: (_, row) => qty(row.salesQuantity), align: 'right', width: 110 },
                { title: '销售额', render: (_, row) => money(row.salesAmount), align: 'right', width: 120 },
                { title: '成本', render: (_, row) => money(row.costAmount), align: 'right', width: 120 },
                { title: '利润', render: (_, row) => money(row.profitAmount), align: 'right', width: 120 },
                { title: '赠品数量', render: (_, row) => qty(row.giftQuantity), align: 'right', width: 110 },
                { title: '赠品成本', render: (_, row) => money(row.giftCostAmount), align: 'right', width: 120 }
              ]}
            />
          </Card>
        </>
      )}
    </div>
  );
}
