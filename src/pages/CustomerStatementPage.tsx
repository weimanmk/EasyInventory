import { App, Button, Card, DatePicker, Select, Space, Statistic, Table, Tag, Typography } from 'antd';
import dayjs, { type Dayjs } from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { money } from '../shared/format';
import type { CustomerStatementDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

type StatementRange = [Dayjs, Dayjs];

const defaultRange = (): StatementRange => [dayjs().startOf('month'), dayjs()];

export default function CustomerStatementPage() {
  const { message } = App.useApp();
  const { customers, terms } = useAppStore();
  const [customerId, setCustomerId] = useState<number>();
  const [range, setRange] = useState<StatementRange>(defaultRange);
  const [statement, setStatement] = useState<CustomerStatementDto>();
  const [loading, setLoading] = useState(false);
  const customerOptions = useMemo(
    () => customers.map((item) => ({ value: item.id, label: `${item.region ? `${item.region} / ` : ''}${item.name}` })),
    [customers]
  );

  async function loadStatement(nextCustomerId = customerId) {
    if (!nextCustomerId) {
      message.warning(`请选择${terms.customer}`);
      return;
    }
    setLoading(true);
    try {
      const result = await api.customerStatement({
        customerId: nextCustomerId,
        startDate: range[0].format('YYYY-MM-DD'),
        endDate: range[1].format('YYYY-MM-DD')
      });
      setStatement(result);
    } catch (error) {
      message.error(error instanceof Error ? error.message : `${terms.customer}对账单查询失败`);
    } finally {
      setLoading(false);
    }
  }

  async function exportStatement() {
    if (!customerId) {
      message.warning(`请选择${terms.customer}`);
      return;
    }
    setLoading(true);
    try {
      const path = await api.exportData({
        exportType: 'customer_statement',
        customerId,
        startDate: range[0].format('YYYY-MM-DD'),
        endDate: range[1].format('YYYY-MM-DD')
      });
      message.success(`已导出：${path}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : `${terms.customer}对账单导出失败`);
    } finally {
      setLoading(false);
    }
  }

  async function exportStatementPdf() {
    if (!customerId) {
      message.warning(`请选择${terms.customer}`);
      return;
    }
    setLoading(true);
    try {
      const path = await api.exportCustomerStatementPdf({
        customerId,
        startDate: range[0].format('YYYY-MM-DD'),
        endDate: range[1].format('YYYY-MM-DD')
      });
      message.success(`已导出 PDF：${path}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : `${terms.customer}对账单 PDF 导出失败`);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!customerId && customers.length > 0) {
      setCustomerId(customers[0].id);
    }
  }, [customerId, customers]);

  const summary = statement?.summary;

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>{terms.customer}对账单</Typography.Title>
          <Typography.Text type="secondary">{terms.customer}应收、收款和余额明细</Typography.Text>
        </div>
        <Space>
          <Button loading={loading} onClick={() => void loadStatement()}>查询</Button>
          <Button type="primary" loading={loading} disabled={!statement} onClick={() => void exportStatement()}>
            导出 Excel
          </Button>
          <Button loading={loading} disabled={!statement} onClick={() => void exportStatementPdf()}>
            导出 PDF
          </Button>
        </Space>
      </div>

      <Card>
        <Space wrap size={12}>
          <Select
            showSearch
            optionFilterProp="label"
            placeholder={terms.customer}
            value={customerId}
            style={{ width: 260 }}
            options={customerOptions}
            onChange={(value) => {
              setCustomerId(value);
              setStatement(undefined);
            }}
          />
          <DatePicker.RangePicker
            value={range}
            onChange={(values) => {
              if (values?.[0] && values[1]) {
                setRange([values[0], values[1]]);
                setStatement(undefined);
              }
            }}
          />
          <Button loading={loading} onClick={() => void loadStatement()}>刷新</Button>
        </Space>
      </Card>

      <div className="stat-grid">
        <Card><Statistic title="期初欠款" value={money(summary?.openingBalance)} /></Card>
        <Card><Statistic title="本期应收" value={money(summary?.periodPayable)} /></Card>
        <Card><Statistic title="本期收款" value={money(summary?.periodPaid)} /></Card>
        <Card><Statistic title="本期优惠" value={money(summary?.periodDiscountAmount)} /></Card>
        <Card><Statistic title="期末余额" value={money(summary?.closingBalance)} valueStyle={{ color: (summary?.closingBalance ?? 0) > 0 ? '#d4380d' : '#16a34a' }} /></Card>
      </div>

      <Card title={summary ? `${summary.customerName} / ${summary.startDate} 至 ${summary.endDate}` : '对账流水'}>
        <Table
          rowKey={(row) => `${row.recordDate}-${row.recordType}-${row.recordNo}`}
          size="small"
          loading={loading}
          dataSource={statement?.rows ?? []}
          columns={[
            { title: '日期', dataIndex: 'recordDate', width: 110 },
            {
              title: '类型',
              width: 90,
              render: (_, row) => (
                <Tag color={row.recordType === 'order' ? 'blue' : 'green'}>
                  {row.recordType === 'order' ? '出库' : '收款'}
                </Tag>
              )
            },
            { title: '单号', dataIndex: 'recordNo', width: 140 },
            { title: '说明', dataIndex: 'description', width: 120 },
            { title: '应收', render: (_, row) => money(row.debitAmount), align: 'right', width: 120 },
            { title: '收款', render: (_, row) => money(row.creditAmount), align: 'right', width: 120 },
            { title: '余额', render: (_, row) => money(row.balanceAfter), align: 'right', width: 120 },
            { title: '备注', dataIndex: 'remark' }
          ]}
        />
      </Card>
    </div>
  );
}
