import { DeleteOutlined, FileExcelOutlined, PrinterOutlined, ShoppingCartOutlined, WalletOutlined } from '@ant-design/icons';
import { App, Button, Card, DatePicker, Form, Input, InputNumber, Modal, Select, Space, Statistic, Table, Tag, Typography } from 'antd';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { api } from '../api/inventory';
import ProductPickerModal from '../components/ProductPickerModal';
import { money, uniqueValues } from '../shared/format';
import type { CustomerDto, MonthlyCreditDto, MonthlyCreditUse, OrderLine } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function OutboundPage() {
  const { message, modal } = App.useApp();
  const [form] = Form.useForm();
  const { customers, features, terms, setProducts } = useAppStore();
  const [region, setRegion] = useState<string>();
  const [customer, setCustomer] = useState<CustomerDto>();
  const [lines, setLines] = useState<OrderLine[]>([]);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [creditLine, setCreditLine] = useState<OrderLine | null>(null);
  const [availableCredits, setAvailableCredits] = useState<MonthlyCreditDto[]>([]);
  const [saving, setSaving] = useState(false);

  const regions = useMemo(() => uniqueValues(customers, (item) => item.region), [customers]);
  const filteredCustomers = useMemo(
    () => customers.filter((item) => !region || item.region === region),
    [customers, region]
  );
  const totals = useMemo(() => {
    const productSalesAmount = lines.reduce((sum, item) => sum + item.amount, 0);
    const directDiscountAmount = lines.reduce((sum, item) => sum + (item.preview?.directDiscountPreview?.amount ?? 0), 0);
    const brandSubsidyAmount = features.monthlyCredit
      ? lines.reduce((sum, item) => sum + (item.preview?.monthlyCreditPreview?.amount ?? 0), 0)
      : 0;
    const monthlyCreditUsed = features.monthlyCredit
      ? lines.reduce(
        (sum, item) => sum + (item.monthlyCreditUses ?? []).reduce((inner, credit) => inner + credit.amount, 0),
        0
      )
      : 0;
    const customerPayableAmount = productSalesAmount - directDiscountAmount - monthlyCreditUsed;
    return { productSalesAmount, directDiscountAmount, brandSubsidyAmount, monthlyCreditUsed, customerPayableAmount };
  }, [features.monthlyCredit, lines]);

  function addLine(line: OrderLine) {
    setLines((prev) => [...prev, line]);
  }

  async function refreshLine(line: OrderLine, patch: Partial<Pick<OrderLine, 'quantity' | 'unitPrice' | 'remark'>>) {
    const next = { ...line, ...patch };
    if (!customer) {
      setLines((prev) => prev.map((item) => item.key === line.key ? next : item));
      return;
    }
    try {
      const preview = await api.previewQuote({
        customerId: customer.id,
        productId: next.productId,
        quantity: next.quantity,
        manualPrice: next.unitPrice,
        orderDate: form.getFieldValue('orderDate')?.format('YYYY-MM-DD') ?? dayjs().format('YYYY-MM-DD')
      });
      const refreshed = {
        ...next,
        amount: Number((next.quantity * next.unitPrice).toFixed(2)),
        ruleMessage: preview.message,
        preview
      };
      setLines((prev) => prev.map((item) => item.key === line.key ? refreshed : item));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '明细重算失败');
    }
  }

  async function openCreditModal(line: OrderLine) {
    if (!features.monthlyCredit) {
      message.warning(`${terms.credit}功能已关闭`);
      return;
    }
    if (!customer) {
      message.warning(`请先选择${terms.customer}`);
      return;
    }
    try {
      const credits = await api.availableMonthlyCredits(
        customer.id,
        line.category,
        form.getFieldValue('orderDate')?.format('YYYY-MM-DD') ?? dayjs().format('YYYY-MM-DD')
      );
      setAvailableCredits(credits);
      setCreditLine(line);
    } catch (error) {
      message.error(error instanceof Error ? error.message : `可用${terms.credit}加载失败`);
    }
  }

  function updateCreditUse(credit: MonthlyCreditDto, amount: number | null) {
    if (!creditLine) {
      return;
    }
    const nextAmount = Number(amount ?? 0);
    setLines((prev) => prev.map((line) => {
      if (line.key !== creditLine.key) {
        return line;
      }
      const rest = (line.monthlyCreditUses ?? []).filter((item) => item.monthlyCreditId !== credit.id);
      const nextUses: MonthlyCreditUse[] = nextAmount > 0
        ? [...rest, { monthlyCreditId: credit.id, amount: nextAmount }]
        : rest;
      const nextLine = { ...line, monthlyCreditUses: nextUses };
      setCreditLine(nextLine);
      return nextLine;
    }));
  }

  async function saveOrder(printAfter = false) {
    const values = await form.validateFields();
    if (!customer) {
      message.warning(`请选择${terms.customer}`);
      return;
    }
    if (lines.length === 0) {
      message.warning(`请选择${terms.product}`);
      return;
    }
    setSaving(true);
    try {
      const response = await api.saveOrder({
        orderDate: values.orderDate.format('YYYY-MM-DD'),
        customerId: customer.id,
        customerAddress: values.address,
        remark: values.remark,
        items: lines.map((line) => ({
          productId: line.productId,
          quantity: line.quantity,
          unitPrice: line.unitPrice,
          remark: line.remark,
          monthlyCreditUses: features.monthlyCredit ? line.monthlyCreditUses : undefined
        }))
      }) as { orderId: number; orderNo: string; documentPath: string };
      if (printAfter) {
        const settings = await api.settings().catch(() => []);
        const defaultPrinter = settings.find((item) => item.key === 'default_printer')?.value || undefined;
        const result = await api.printOrderWithOptions(response.orderId, { printerName: defaultPrinter });
        message.info(result.message);
      }
      message.success(`保存成功：${response.orderNo}`);
      setLines([]);
      const products = await api.products({ isActive: true });
      setProducts(products);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '保存失败');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>快速出库</Typography.Title>
          <Typography.Text type="secondary">
            {terms.customer}、{terms.product}、数量、价格集中处理
          </Typography.Text>
        </div>
        <Space>
          <Button icon={<FileExcelOutlined />} onClick={() => void saveOrder(false)} loading={saving}>保存并导出</Button>
          <Button type="primary" icon={<PrinterOutlined />} onClick={() => void saveOrder(true)} loading={saving}>保存并打印</Button>
        </Space>
      </div>
      <Form
        form={form}
        layout="vertical"
        initialValues={{ orderDate: dayjs() }}
        className="dense-form panel"
      >
        <div style={{ display: 'grid', gridTemplateColumns: '160px 160px 240px minmax(0,1fr)', gap: 12 }}>
          <Form.Item label="日期" name="orderDate" rules={[{ required: true }]}>
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label={terms.region}>
            <Select
              allowClear
              options={regions.map((value) => ({ value, label: value }))}
              value={region}
              onChange={setRegion}
            />
          </Form.Item>
          <Form.Item label={terms.customer} rules={[{ required: true }]}>
            <Select
              showSearch
              optionFilterProp="label"
              options={filteredCustomers.map((item) => ({ value: item.id, label: item.name }))}
              onChange={(id) => {
                const next = customers.find((item) => item.id === id);
                setCustomer(next);
                form.setFieldValue('address', next?.address);
              }}
            />
          </Form.Item>
          <Form.Item label="地址" name="address">
            <Input />
          </Form.Item>
        </div>
        <Form.Item label="备注" name="remark">
          <Input />
        </Form.Item>
      </Form>
      <div className="outbound-grid">
        <Card
          title="出库明细"
          extra={<Button type="primary" icon={<ShoppingCartOutlined />} onClick={() => setPickerOpen(true)}>选择{terms.product}</Button>}
        >
          <Table
            className="outbound-lines-table"
            rowKey="key"
            dataSource={lines}
            pagination={false}
            size="small"
            scroll={{ x: 1180 }}
            columns={[
              { title: '类型', render: () => <Tag color="blue">正常</Tag>, width: 76 },
              {
                title: terms.product,
                dataIndex: 'productName',
                width: 220,
                render: (value) => (
                  <Typography.Text className="outbound-cell-ellipsis" ellipsis={{ tooltip: value }}>
                    {value}
                  </Typography.Text>
                )
              },
              {
                title: terms.category,
                dataIndex: 'category',
                width: 110,
                render: (value) => (
                  <Typography.Text className="outbound-cell-ellipsis" ellipsis={{ tooltip: value }}>
                    {value || '-'}
                  </Typography.Text>
                )
              },
              { title: '条码', dataIndex: 'barcode', width: 150 },
              { title: '库存', dataIndex: 'currentStock', align: 'right', width: 80 },
              {
                title: '数量',
                dataIndex: 'quantity',
                align: 'right',
                width: 112,
                render: (_, row) => (
                  <InputNumber
                    min={0.01}
                    value={row.quantity}
                    size="small"
                    style={{ width: 88 }}
                    onChange={(value) => void refreshLine(row, { quantity: Number(value ?? 1) })}
                  />
                )
              },
              {
                title: '单价',
                align: 'right',
                width: 112,
                render: (_, row) => (
                  <InputNumber
                    min={0}
                    value={row.unitPrice}
                    size="small"
                    style={{ width: 88 }}
                    onChange={(value) => void refreshLine(row, { unitPrice: Number(value ?? 0) })}
                  />
                )
              },
              { title: '金额', render: (_, row) => money(row.amount), align: 'right', width: 100 },
              {
                title: '备注',
                width: 150,
                render: (_, row) => (
                  <Input
                    size="small"
                    value={row.remark}
                    onChange={(event) => void refreshLine(row, { remark: event.target.value })}
                  />
                )
              },
              ...(features.monthlyCredit
                ? [{
                  title: `${terms.credit}抵扣`,
                  width: 130,
                  render: (_: unknown, row: OrderLine) => {
                    const used = (row.monthlyCreditUses ?? []).reduce((sum, item) => sum + item.amount, 0);
                    return (
                      <Button size="small" icon={<WalletOutlined />} onClick={() => void openCreditModal(row)}>
                        {used > 0 ? money(used) : '选择'}
                      </Button>
                    );
                  }
                }]
                : []),
              {
                title: '规则',
                width: 180,
                render: (_, row) => row.ruleMessage
                  ? (
                    <Typography.Text className="outbound-cell-ellipsis" ellipsis={{ tooltip: row.ruleMessage }}>
                      {row.ruleMessage}
                    </Typography.Text>
                  )
                  : '-'
              },
              {
                title: '操作',
                width: 74,
                render: (_, row) => (
                  <Button
                    danger
                    type="text"
                    icon={<DeleteOutlined />}
                    onClick={() => setLines((prev) => prev.filter((item) => item.key !== row.key))}
                  />
                )
              }
            ]}
          />
        </Card>
        <Card title="合计">
          <div className="totals-panel">
            <Statistic title={`${terms.product}销售额`} value={money(totals.productSalesAmount)} />
            <Statistic title="本单折现" value={money(totals.directDiscountAmount)} valueStyle={{ color: '#d4380d' }} />
            {features.monthlyCredit && (
              <>
                <Statistic title={`${terms.credit}抵扣`} value={money(totals.monthlyCreditUsed)} valueStyle={{ color: '#d4380d' }} />
                <Statistic title={`生成${terms.credit}`} value={money(totals.brandSubsidyAmount)} />
              </>
            )}
            <Statistic title={`${terms.customer}实收`} value={money(totals.customerPayableAmount)} valueStyle={{ color: '#16a34a' }} />
            <Button block onClick={() => modal.confirm({ title: '清空出库明细？', onOk: () => setLines([]) })}>清空</Button>
          </div>
        </Card>
      </div>
      <ProductPickerModal open={pickerOpen} customer={customer} onClose={() => setPickerOpen(false)} onAdd={addLine} />
      <Modal
        title={creditLine ? `选择${terms.credit}抵扣：${creditLine.productName}` : `选择${terms.credit}抵扣`}
        open={features.monthlyCredit && !!creditLine}
        onCancel={() => setCreditLine(null)}
        onOk={() => setCreditLine(null)}
        width={720}
      >
        <Table
          rowKey="id"
          dataSource={availableCredits}
          pagination={false}
          size="small"
          columns={[
            { title: '来源订单', dataIndex: 'sourceOrderNo' },
            { title: terms.category, dataIndex: 'category', width: 100 },
            { title: '可用月份', dataIndex: 'availableMonth', width: 100 },
            { title: '剩余', render: (_, row) => money(row.remainingAmount), align: 'right', width: 90 },
            {
              title: '本单抵扣',
              width: 140,
              render: (_, row) => {
                const current = creditLine?.monthlyCreditUses?.find((item) => item.monthlyCreditId === row.id)?.amount ?? 0;
                return (
                  <InputNumber
                    min={0}
                    max={row.remainingAmount}
                    value={current}
                    size="small"
                    style={{ width: 110 }}
                    onChange={(value) => updateCreditUse(row, value)}
                  />
                );
              }
            }
          ]}
          locale={{ emptyText: `暂无可用${terms.credit}` }}
        />
      </Modal>
    </div>
  );
}
