import { Alert, App, Button, Card, DatePicker, Select, Space, Statistic, Table, Typography } from 'antd';
import type { EChartsOption } from 'echarts';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import EChart from '../components/EChart';
import { money, qty } from '../shared/format';
import type { SupplierDto, SupplierPurchaseLedgerDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

const defaultRange = (): [string, string] => [
  dayjs().startOf('year').format('YYYY-MM-DD'),
  dayjs().format('YYYY-MM-DD')
];

export default function SupplierLedgerPage() {
  const { message } = App.useApp();
  const { terms, features } = useAppStore();
  const [suppliers, setSuppliers] = useState<SupplierDto[]>([]);
  const [ledger, setLedger] = useState<SupplierPurchaseLedgerDto>({
    summaries: [],
    details: [],
    monthlyTrend: []
  });
  const [range, setRange] = useState<[string, string]>(defaultRange);
  const [supplierId, setSupplierId] = useState<number>();
  const [loading, setLoading] = useState(false);
  const totalAmount = ledger.summaries.reduce((sum, row) => sum + row.inboundAmount, 0);
  const totalCount = ledger.summaries.reduce((sum, row) => sum + row.inboundCount, 0);
  const recentDates = ledger.summaries
    .map((row) => row.recentInboundDate)
    .filter(Boolean)
    .sort();
  const recentDate = recentDates.length > 0 ? recentDates[recentDates.length - 1] : undefined;
  const trendOption = useMemo<EChartsOption>(() => ({
    tooltip: { trigger: 'axis' },
    grid: { left: 48, right: 24, top: 32, bottom: 36 },
    xAxis: { type: 'category', data: ledger.monthlyTrend.map((item) => item.period) },
    yAxis: [
      { type: 'value', name: '金额' },
      { type: 'value', name: '次数' }
    ],
    series: [
      {
        name: '采购金额',
        type: 'bar',
        data: ledger.monthlyTrend.map((item) => item.inboundAmount),
        itemStyle: { color: '#2563eb' }
      },
      {
        name: '入库次数',
        type: 'line',
        yAxisIndex: 1,
        data: ledger.monthlyTrend.map((item) => item.inboundCount),
        smooth: true,
        itemStyle: { color: '#16a34a' }
      }
    ]
  }), [ledger.monthlyTrend]);

  async function loadLedger(nextSupplierId?: number, useCurrentSupplier = true) {
    if (!features.supplierLedger) {
      setLedger({ summaries: [], details: [], monthlyTrend: [] });
      return;
    }
    const resolvedSupplierId = useCurrentSupplier ? (nextSupplierId ?? supplierId) : nextSupplierId;
    setLoading(true);
    try {
      const result = await api.supplierPurchaseLedger({
        startDate: range[0],
        endDate: range[1],
        supplierId: resolvedSupplierId
      });
      setLedger(result);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '采购台账加载失败');
    } finally {
      setLoading(false);
    }
  }

  async function boot() {
    if (!features.supplierLedger) {
      setSuppliers([]);
      setLedger({ summaries: [], details: [], monthlyTrend: [] });
      return;
    }
    try {
      const [nextSuppliers, nextLedger] = await Promise.all([
        api.suppliers({ isActive: true }),
        api.supplierPurchaseLedger({ startDate: range[0], endDate: range[1] })
      ]);
      setSuppliers(nextSuppliers);
      setLedger(nextLedger);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '供应商采购台账初始化失败');
    }
  }

  useEffect(() => {
    void boot();
  }, [features.supplierLedger]);

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>供应商采购台账</Typography.Title>
          <Typography.Text type="secondary">供应商入库金额、明细和月度趋势</Typography.Text>
        </div>
        <Button type="primary" loading={loading} onClick={() => void loadLedger()}>查询</Button>
      </div>

      {!features.supplierLedger && (
        <Alert
          type="info"
          showIcon
          message="供应商采购台账功能已关闭"
          description="可以在系统设置的功能开关中重新开启，历史供应商和入库数据会保留。"
        />
      )}
      {features.supplierLedger && (
        <>
          <div className="toolbar panel">
            <DatePicker.RangePicker
              value={[dayjs(range[0]), dayjs(range[1])]}
              onChange={(values) => {
                if (values) {
                  setRange([values[0]!.format('YYYY-MM-DD'), values[1]!.format('YYYY-MM-DD')]);
                }
              }}
            />
            <Select
              allowClear
              showSearch
              optionFilterProp="label"
              placeholder="供应商"
              value={supplierId}
              style={{ width: 240 }}
              options={suppliers.map((item) => ({ value: item.id, label: item.name }))}
              onChange={setSupplierId}
            />
            <Button onClick={() => {
              setSupplierId(undefined);
              void loadLedger(undefined, false);
            }}>
              全部供应商
            </Button>
          </div>

          <div className="stat-grid">
            <Card><Statistic title="采购金额" value={money(totalAmount)} /></Card>
            <Card><Statistic title="入库次数" value={totalCount} /></Card>
            <Card><Statistic title="供应商数" value={ledger.summaries.length} /></Card>
            <Card><Statistic title="最近入库" value={recentDate ?? '-'} /></Card>
          </div>

          <Card title="采购金额月度趋势">
            <EChart option={trendOption} height={300} empty={ledger.monthlyTrend.length === 0} />
          </Card>

          <Card title="供应商汇总">
            <Table
              rowKey={(row) => `${row.supplierId ?? 'none'}-${row.supplierName}`}
              size="small"
              loading={loading}
              dataSource={ledger.summaries}
              columns={[
                { title: '供应商', dataIndex: 'supplierName' },
                { title: '入库次数', dataIndex: 'inboundCount', align: 'right', width: 120 },
                { title: '采购金额', render: (_, row) => money(row.inboundAmount), align: 'right', width: 140 },
                { title: '最近入库', dataIndex: 'recentInboundDate', width: 130 },
                {
                  title: '操作',
                  width: 100,
                  render: (_, row) => (
                    <Button
                      size="small"
                      disabled={!row.supplierId}
                      onClick={() => {
                        setSupplierId(row.supplierId);
                        void loadLedger(row.supplierId);
                      }}
                    >
                      查看明细
                    </Button>
                  )
                }
              ]}
            />
          </Card>

          <Card title="入库明细">
            <Table
              rowKey="id"
              size="small"
              loading={loading}
              dataSource={ledger.details}
              scroll={{ x: 980 }}
              columns={[
                { title: '日期', dataIndex: 'inboundDate', width: 110 },
                { title: '供应商', dataIndex: 'supplierName', width: 180 },
                { title: terms.product, dataIndex: 'productName', width: 200 },
                { title: terms.category, dataIndex: 'category', width: 120 },
                { title: '数量', render: (_, row) => qty(row.quantity), align: 'right', width: 100 },
                { title: '进货价', render: (_, row) => money(row.unitCost), align: 'right', width: 120 },
                { title: '金额', render: (_, row) => money(row.amount), align: 'right', width: 120 },
                { title: '备注', dataIndex: 'remark' }
              ]}
            />
          </Card>
        </>
      )}
    </div>
  );
}
