import { Alert, App, Button, Card, Checkbox, DatePicker, Drawer, Form, Input, InputNumber, Select, Space, Statistic, Table, Tag, Typography } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { money, uniqueValues } from '../shared/format';
import type { CustomerBalanceDto, PaymentRecordDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function ReceivablesPage() {
  const { message, modal } = App.useApp();
  const [form] = Form.useForm();
  const { customers, terms, features } = useAppStore();
  const [balances, setBalances] = useState<CustomerBalanceDto[]>([]);
  const [payments, setPayments] = useState<PaymentRecordDto[]>([]);
  const [region, setRegion] = useState<string>();
  const [keyword, setKeyword] = useState('');
  const [onlyUnpaid, setOnlyUnpaid] = useState(true);
  const [customerId, setCustomerId] = useState<number>();
  const [paymentStatus, setPaymentStatus] = useState('normal');
  const [drawerOpen, setDrawerOpen] = useState(false);
  const regions = useMemo(() => uniqueValues(customers, (item) => item.region), [customers]);
  const totalBalance = balances.reduce((sum, row) => sum + row.balance, 0);
  const totalPaid = payments
    .filter((row) => row.status === 'normal')
    .reduce((sum, row) => sum + row.amount, 0);

  async function loadBalances() {
    if (!features.receivables) {
      setBalances([]);
      return;
    }
    setBalances(await api.customerBalances({ region, keyword, onlyUnpaid }));
  }

  async function loadPayments() {
    if (!features.receivables) {
      setPayments([]);
      return;
    }
    setPayments(await api.paymentRecords({ customerId, status: paymentStatus }));
  }

  async function refreshAll() {
    await Promise.all([loadBalances(), loadPayments()]);
  }

  useEffect(() => {
    void loadBalances().catch((error) => {
      message.warning(error instanceof Error ? error.message : '欠款余额加载失败');
    });
  }, [features.receivables, message, region, keyword, onlyUnpaid]);

  useEffect(() => {
    void loadPayments().catch((error) => {
      message.warning(error instanceof Error ? error.message : '收款记录加载失败');
    });
  }, [features.receivables, message, customerId, paymentStatus]);

  function openPayment(customer?: CustomerBalanceDto) {
    form.resetFields();
    form.setFieldsValue({
      paymentDate: dayjs(),
      customerId: customer?.customerId ?? customerId,
      amount: customer && customer.balance > 0 ? Number(customer.balance.toFixed(2)) : undefined,
      method: '现金'
    });
    setDrawerOpen(true);
  }

  async function savePayment() {
    const values = await form.validateFields();
    try {
      const saved = await api.createPayment({
        paymentDate: values.paymentDate.format('YYYY-MM-DD'),
        customerId: values.customerId,
        amount: values.amount,
        method: values.method,
        relatedOrderId: values.relatedOrderId,
        remark: values.remark
      });
      setDrawerOpen(false);
      setCustomerId(saved.customerId);
      await refreshAll();
      message.success(`收款已登记：${saved.customerName} ${money(saved.amount)}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '收款保存失败');
    }
  }

  async function voidPayment(row: PaymentRecordDto) {
    try {
      await api.voidPayment(row.id);
      await refreshAll();
      message.success('收款已作废');
    } catch (error) {
      message.error(error instanceof Error ? error.message : '作废失败');
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>欠款 / 收款管理</Typography.Title>
          <Typography.Text type="secondary">按{terms.customer}查看应收余额并登记收款</Typography.Text>
        </div>
        <Button type="primary" disabled={!features.receivables} onClick={() => openPayment()}>登记收款</Button>
      </div>
      {!features.receivables && (
        <Alert
          type="info"
          showIcon
          message="欠款收款功能已关闭"
          description={`可以在系统设置的功能开关中重新开启，历史${terms.customer}欠款和收款记录会保留。`}
        />
      )}
      {features.receivables && (
        <>
      <div className="stat-grid">
        <Card><Statistic title="当前筛选欠款" value={money(totalBalance)} valueStyle={{ color: totalBalance > 0 ? '#d4380d' : '#16a34a' }} /></Card>
        <Card><Statistic title={`${terms.customer}数`} value={balances.length} /></Card>
        <Card><Statistic title="当前收款合计" value={money(totalPaid)} /></Card>
      </div>
      <Card title={`${terms.customer}欠款余额`}>
        <div className="toolbar" style={{ marginBottom: 12 }}>
          <Select allowClear placeholder={terms.region} value={region} style={{ width: 160 }} options={regions.map((item) => ({ value: item, label: item }))} onChange={setRegion} />
          <Input allowClear placeholder={`搜索${terms.customer}或地址`} value={keyword} onChange={(event) => setKeyword(event.target.value)} style={{ width: 260 }} />
          <Checkbox checked={onlyUnpaid} onChange={(event) => setOnlyUnpaid(event.target.checked)}>只看欠款</Checkbox>
          <Button onClick={() => void loadBalances()}>刷新余额</Button>
        </div>
        <Table
          rowKey="customerId"
          dataSource={balances}
          size="small"
          columns={[
            { title: terms.customer, dataIndex: 'customerName' },
            { title: terms.region, dataIndex: 'region', width: 120 },
            { title: '应收', render: (_, row) => money(row.totalPayable), align: 'right', width: 120 },
            { title: '已收', render: (_, row) => money(row.totalPaid), align: 'right', width: 120 },
            { title: '余额', render: (_, row) => money(row.balance), align: 'right', width: 120 },
            { title: '最近出库', dataIndex: 'lastOrderDate', width: 120 },
            { title: '最近收款', dataIndex: 'lastPaymentDate', width: 120 },
            {
              title: '操作',
              render: (_, row) => <Button size="small" onClick={() => openPayment(row)}>收款</Button>,
              width: 90
            }
          ]}
        />
      </Card>
      <Card title="收款记录">
        <div className="toolbar" style={{ marginBottom: 12 }}>
          <Select
            allowClear
            showSearch
            optionFilterProp="label"
            placeholder={terms.customer}
            value={customerId}
            style={{ width: 220 }}
            options={customers.map((item) => ({ value: item.id, label: item.name }))}
            onChange={setCustomerId}
          />
          <Select
            value={paymentStatus}
            style={{ width: 130 }}
            options={[
              { value: 'normal', label: '正常' },
              { value: 'voided', label: '已作废' },
              { value: '全部', label: '全部' }
            ]}
            onChange={setPaymentStatus}
          />
          <Button onClick={() => void loadPayments()}>刷新记录</Button>
        </div>
        <Table
          rowKey="id"
          dataSource={payments}
          size="small"
          columns={[
            { title: '日期', dataIndex: 'paymentDate', width: 110 },
            { title: terms.customer, dataIndex: 'customerName' },
            { title: '金额', render: (_, row) => money(row.amount), align: 'right', width: 120 },
            { title: '方式', dataIndex: 'method', width: 100 },
            { title: '关联订单ID', dataIndex: 'relatedOrderId', width: 120 },
            { title: '状态', render: (_, row) => <Tag color={row.status === 'normal' ? 'green' : 'default'}>{row.status === 'normal' ? '正常' : '已作废'}</Tag>, width: 90 },
            { title: '备注', dataIndex: 'remark' },
            {
              title: '操作',
              render: (_, row) => (
                <Button
                  size="small"
                  danger
                  disabled={row.status !== 'normal'}
                  onClick={() => modal.confirm({
                    title: '作废该收款？',
                    content: `作废后该笔收款不再抵扣${terms.customer}欠款。`,
                    okText: '作废',
                    okButtonProps: { danger: true },
                    onOk: () => void voidPayment(row)
                  })}
                >
                  作废
                </Button>
              ),
              width: 90
            }
          ]}
        />
      </Card>
      <Drawer title="登记收款" open={drawerOpen} onClose={() => setDrawerOpen(false)} width={420}>
        <Form form={form} layout="vertical" className="dense-form">
          <Form.Item label="收款日期" name="paymentDate" rules={[{ required: true, message: '请选择收款日期' }]}>
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label={terms.customer} name="customerId" rules={[{ required: true, message: `请选择${terms.customer}` }]}>
            <Select showSearch optionFilterProp="label" options={customers.map((item) => ({ value: item.id, label: item.name }))} />
          </Form.Item>
          <Form.Item label="金额" name="amount" rules={[{ required: true, message: '请输入收款金额' }]}>
            <InputNumber min={0.01} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label="收款方式" name="method">
            <Select
              options={[
                { value: '现金', label: '现金' },
                { value: '微信', label: '微信' },
                { value: '支付宝', label: '支付宝' },
                { value: '银行转账', label: '银行转账' },
                { value: '其他', label: '其他' }
              ]}
            />
          </Form.Item>
          <Form.Item label="关联订单ID" name="relatedOrderId"><InputNumber min={1} style={{ width: '100%' }} /></Form.Item>
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" block onClick={() => void savePayment()}>保存收款</Button>
        </Form>
      </Drawer>
      </>
      )}
    </div>
  );
}
