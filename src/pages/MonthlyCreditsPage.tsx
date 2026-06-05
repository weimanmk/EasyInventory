import { Alert, App, Button, Select, Space, Table, Tag, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { money, uniqueValues } from '../shared/format';
import type { MonthlyCreditDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

const statusMap: Record<string, { label: string; color: string }> = {
  pending: { label: '未到期', color: 'orange' },
  available: { label: '可用', color: 'green' },
  used_up: { label: '已用完', color: 'default' },
  closed: { label: '关闭', color: 'red' },
  voided: { label: '作废', color: 'red' }
};

export default function MonthlyCreditsPage() {
  const { message, modal } = App.useApp();
  const { customers, products, terms, features } = useAppStore();
  const [rows, setRows] = useState<MonthlyCreditDto[]>([]);
  const [customerId, setCustomerId] = useState<number>();
  const [category, setCategory] = useState<string>();
  const [status, setStatus] = useState<string>();
  const categories = useMemo(() => uniqueValues(products, (item) => item.category), [products]);

  async function load() {
    if (!features.monthlyCredit) {
      setRows([]);
      return;
    }
    try {
      setRows(await api.monthlyCredits({ customerId, category, status }));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '加载失败');
    }
  }

  useEffect(() => {
    void load();
  }, [features.monthlyCredit]);

  return (
    <div className="page">
      <div className="page-title"><Typography.Title level={2}>{terms.credit}账本</Typography.Title></div>
      {!features.monthlyCredit && (
        <Alert
          type="info"
          showIcon
          message={`${terms.credit}功能已关闭`}
          description={`可以在系统设置的功能开关中重新开启，历史${terms.credit}数据会保留。`}
        />
      )}
      {features.monthlyCredit && (
        <>
      <div className="toolbar panel">
        <Select allowClear showSearch optionFilterProp="label" placeholder={terms.customer} style={{ width: 220 }} options={customers.map((item) => ({ value: item.id, label: item.name }))} onChange={setCustomerId} />
        <Select allowClear placeholder={terms.category} style={{ width: 160 }} options={categories.map((item) => ({ value: item, label: item }))} onChange={setCategory} />
        <Select allowClear placeholder="状态" style={{ width: 140 }} options={Object.entries(statusMap).map(([value, item]) => ({ value, label: item.label }))} onChange={setStatus} />
        <Button onClick={() => void load()}>查询</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={rows}
        columns={[
          { title: '来源订单', dataIndex: 'sourceOrderNo' },
          { title: terms.customer, dataIndex: 'customerName' },
          { title: terms.category, dataIndex: 'category', width: 100 },
          { title: '生成金额', render: (_, row) => money(row.amount), align: 'right' },
          { title: '已使用', render: (_, row) => money(row.usedAmount), align: 'right' },
          { title: '剩余', render: (_, row) => money(row.remainingAmount), align: 'right' },
          { title: '生成日期', dataIndex: 'generatedDate' },
          { title: '可用月份', dataIndex: 'availableMonth' },
          { title: '状态', render: (_, row) => <Tag color={statusMap[row.status]?.color}>{statusMap[row.status]?.label ?? row.status}</Tag> },
          {
            title: '操作',
            render: (_, row) => (
              <Space>
                <Button size="small" disabled={row.status === 'closed'} onClick={() => modal.confirm({ title: `关闭该${terms.credit}？`, onOk: async () => { await api.closeMonthlyCredit(row.id); await load(); } })}>关闭</Button>
                <Button size="small" danger disabled={row.status === 'voided'} onClick={() => modal.confirm({ title: `作废该${terms.credit}？`, onOk: async () => { await api.voidMonthlyCredit(row.id); await load(); } })}>作废</Button>
              </Space>
            )
          }
        ]}
      />
      </>
      )}
    </div>
  );
}
