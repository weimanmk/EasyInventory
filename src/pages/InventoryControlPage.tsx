import {
  Alert,
  App,
  Button,
  Card,
  DatePicker,
  Descriptions,
  Form,
  Input,
  InputNumber,
  Select,
  Space,
  Table,
  Tabs,
  Tag,
  Typography
} from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { money, qty, uniqueValues } from '../shared/format';
import type { InventoryAdjustmentDto, ProductDto, StocktakeRecordDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

const adjustmentTypes = [
  { value: 'loss', label: '损耗' },
  { value: 'increase', label: '补增' },
  { value: 'scrap', label: '报废' },
  { value: 'self_use', label: '自用' },
  { value: 'other', label: '其他' }
];

export default function InventoryControlPage() {
  const { message, modal } = App.useApp();
  const { products, setProducts, terms, features } = useAppStore();
  const [stocktakeForm] = Form.useForm();
  const [adjustmentForm] = Form.useForm();
  const [category, setCategory] = useState<string>();
  const [scanCode, setScanCode] = useState('');
  const [stocktakes, setStocktakes] = useState<StocktakeRecordDto[]>([]);
  const [adjustments, setAdjustments] = useState<InventoryAdjustmentDto[]>([]);
  const [loading, setLoading] = useState(false);
  const categories = useMemo(() => uniqueValues(products, (item) => item.category), [products]);
  const filteredProducts = useMemo(
    () => products.filter((item) => !category || item.category === category),
    [category, products]
  );
  const stocktakeProduct = products.find((item) => item.id === stocktakeForm.getFieldValue('productId'));
  const adjustmentProduct = products.find((item) => item.id === adjustmentForm.getFieldValue('productId'));

  async function refresh() {
    if (!features.inventoryControl) {
      setStocktakes([]);
      setAdjustments([]);
      return;
    }
    setLoading(true);
    try {
      const [nextStocktakes, nextAdjustments, nextProducts] = await Promise.all([
        api.stocktakes({}),
        api.inventoryAdjustments({}),
        api.products({ isActive: true })
      ]);
      setStocktakes(nextStocktakes);
      setAdjustments(nextAdjustments);
      setProducts(nextProducts);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '库存盘点数据加载失败');
    } finally {
      setLoading(false);
    }
  }

  function locateScannedProduct() {
    const barcode = scanCode.trim();
    if (!barcode) {
      return;
    }
    const product = products.find((item) => item.barcode === barcode);
    if (!product) {
      message.warning(`未找到条码：${barcode}`);
      return;
    }
    setCategory(product.category);
    stocktakeForm.setFieldsValue({ productId: product.id });
    adjustmentForm.setFieldsValue({ productId: product.id });
    setScanCode('');
    message.success(`已定位${terms.product}：${product.name}`);
  }

  async function saveStocktake() {
    const values = await stocktakeForm.validateFields();
    try {
      await api.createStocktake({
        stocktakeDate: values.stocktakeDate.format('YYYY-MM-DD'),
        productId: values.productId,
        actualStock: values.actualStock,
        reason: values.reason,
        remark: values.remark
      });
      message.success('盘点已保存');
      stocktakeForm.setFieldsValue({ actualStock: undefined, reason: undefined, remark: undefined });
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '盘点保存失败');
    }
  }

  async function saveAdjustment() {
    const values = await adjustmentForm.validateFields();
    try {
      await api.createInventoryAdjustment({
        adjustmentDate: values.adjustmentDate.format('YYYY-MM-DD'),
        productId: values.productId,
        adjustmentType: values.adjustmentType,
        quantityDelta: values.quantityDelta,
        reason: values.reason,
        remark: values.remark
      });
      message.success('库存调整已保存');
      adjustmentForm.setFieldsValue({ quantityDelta: undefined, reason: undefined, remark: undefined });
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '库存调整保存失败');
    }
  }

  function voidStocktake(row: StocktakeRecordDto) {
    modal.confirm({
      title: `作废盘点记录 ${row.productName}？`,
      content: '作废后会写入反向库存流水并恢复库存余额。',
      okText: '作废',
      okButtonProps: { danger: true },
      onOk: async () => {
        await api.voidStocktake(row.id, { reason: '界面作废盘点' });
        message.success('盘点记录已作废');
        await refresh();
      }
    });
  }

  function voidAdjustment(row: InventoryAdjustmentDto) {
    modal.confirm({
      title: `作废库存调整 ${row.productName}？`,
      content: '作废后会写入反向库存流水并恢复库存余额。',
      okText: '作废',
      okButtonProps: { danger: true },
      onOk: async () => {
        await api.voidInventoryAdjustment(row.id, { reason: '界面作废库存调整' });
        message.success('库存调整已作废');
        await refresh();
      }
    });
  }

  useEffect(() => {
    void refresh();
  }, [features.inventoryControl]);

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>库存盘点</Typography.Title>
          <Typography.Text type="secondary">录入实盘库存或手工调整，系统自动写入库存流水并重算余额</Typography.Text>
        </div>
        <Button loading={loading} onClick={() => void refresh()}>刷新</Button>
      </div>

      {!features.inventoryControl && (
        <Alert
          type="info"
          showIcon
          message="库存盘点功能已关闭"
          description="可以在系统设置的功能开关中重新开启，历史盘点和库存调整数据会保留。"
        />
      )}
      {features.inventoryControl && (
        <>
          <div className="toolbar panel">
            <Select
              allowClear
              placeholder={`按${terms.category}筛选${terms.product}`}
              style={{ width: 220 }}
              value={category}
              options={categories.map((item) => ({ value: item, label: item }))}
              onChange={setCategory}
            />
            <Input
              allowClear
              placeholder="扫码或输入条码后回车定位"
              style={{ width: 260 }}
              value={scanCode}
              onChange={(event) => setScanCode(event.target.value)}
              onPressEnter={locateScannedProduct}
            />
          </div>

          <Tabs
            items={[
          {
            key: 'stocktake',
            label: '盘点录入',
            children: (
              <div className="two-column">
                <Card title="盘点表单">
                  <Form
                    form={stocktakeForm}
                    layout="vertical"
                    initialValues={{ stocktakeDate: dayjs() }}
                    className="dense-form"
                    onValuesChange={() => stocktakeForm.validateFields(['productId']).catch(() => undefined)}
                  >
                    <Form.Item label="日期" name="stocktakeDate" rules={[{ required: true }]}>
                      <DatePicker style={{ width: '100%' }} />
                    </Form.Item>
                    <Form.Item label={terms.product} name="productId" rules={[{ required: true }]}>
                      <Select
                        showSearch
                        optionFilterProp="label"
                        options={filteredProducts.map((item) => ({ value: item.id, label: productLabel(item) }))}
                      />
                    </Form.Item>
                    {stocktakeProduct ? <ProductSnapshot product={stocktakeProduct} /> : null}
                    <Form.Item label="实盘库存" name="actualStock" rules={[{ required: true }]}>
                      <InputNumber min={0} style={{ width: '100%' }} />
                    </Form.Item>
                    <Form.Item label="盘点原因" name="reason" rules={[{ required: true }]}>
                      <Input />
                    </Form.Item>
                    <Form.Item label="备注" name="remark">
                      <Input />
                    </Form.Item>
                    <Button type="primary" block onClick={() => void saveStocktake()}>保存盘点</Button>
                  </Form>
                </Card>
                <Card title="盘点记录">
                  <Table
                    rowKey="id"
                    size="small"
                    loading={loading}
                    dataSource={stocktakes}
                    columns={[
                      { title: '日期', dataIndex: 'stocktakeDate', width: 110 },
                      { title: terms.product, dataIndex: 'productName' },
                      { title: '账面', render: (_, row) => qty(row.systemStock), align: 'right', width: 90 },
                      { title: '实盘', render: (_, row) => qty(row.actualStock), align: 'right', width: 90 },
                      { title: '差异', render: (_, row) => qty(row.differenceQuantity), align: 'right', width: 90 },
                      { title: '差异金额', render: (_, row) => money(row.differenceAmount), align: 'right', width: 100 },
                      { title: '状态', render: (_, row) => statusTag(row.status), width: 90 },
                      {
                        title: '操作',
                        width: 90,
                        render: (_, row) => (
                          <Button
                            size="small"
                            danger
                            disabled={row.status !== 'normal'}
                            onClick={() => voidStocktake(row)}
                          >
                            作废
                          </Button>
                        )
                      }
                    ]}
                  />
                </Card>
              </div>
            )
          },
          {
            key: 'adjustment',
            label: '库存调整',
            children: (
              <div className="two-column">
                <Card title="调整表单">
                  <Form
                    form={adjustmentForm}
                    layout="vertical"
                    initialValues={{ adjustmentDate: dayjs(), adjustmentType: 'loss' }}
                    className="dense-form"
                    onValuesChange={() => adjustmentForm.validateFields(['productId']).catch(() => undefined)}
                  >
                    <Form.Item label="日期" name="adjustmentDate" rules={[{ required: true }]}>
                      <DatePicker style={{ width: '100%' }} />
                    </Form.Item>
                    <Form.Item label={terms.product} name="productId" rules={[{ required: true }]}>
                      <Select
                        showSearch
                        optionFilterProp="label"
                        options={filteredProducts.map((item) => ({ value: item.id, label: productLabel(item) }))}
                      />
                    </Form.Item>
                    {adjustmentProduct ? <ProductSnapshot product={adjustmentProduct} /> : null}
                    <Form.Item label="调整类型" name="adjustmentType" rules={[{ required: true }]}>
                      <Select options={adjustmentTypes} />
                    </Form.Item>
                    <Form.Item label="调整数量（可正可负）" name="quantityDelta" rules={[{ required: true }]}>
                      <InputNumber style={{ width: '100%' }} />
                    </Form.Item>
                    <Form.Item label="调整原因" name="reason" rules={[{ required: true }]}>
                      <Input />
                    </Form.Item>
                    <Form.Item label="备注" name="remark">
                      <Input />
                    </Form.Item>
                    <Button type="primary" block onClick={() => void saveAdjustment()}>保存调整</Button>
                  </Form>
                </Card>
                <Card title="调整记录">
                  <Table
                    rowKey="id"
                    size="small"
                    loading={loading}
                    dataSource={adjustments}
                    columns={[
                      { title: '日期', dataIndex: 'adjustmentDate', width: 110 },
                      { title: terms.product, dataIndex: 'productName' },
                      { title: '类型', render: (_, row) => adjustmentTypeLabel(row.adjustmentType), width: 90 },
                      { title: '数量', render: (_, row) => qty(row.quantityDelta), align: 'right', width: 90 },
                      { title: '金额', render: (_, row) => money(row.amount), align: 'right', width: 100 },
                      { title: '状态', render: (_, row) => statusTag(row.status), width: 90 },
                      {
                        title: '操作',
                        width: 90,
                        render: (_, row) => (
                          <Button
                            size="small"
                            danger
                            disabled={row.status !== 'normal'}
                            onClick={() => voidAdjustment(row)}
                          >
                            作废
                          </Button>
                        )
                      }
                    ]}
                  />
                </Card>
              </div>
            )
          }
            ]}
          />
        </>
      )}
    </div>
  );
}

function ProductSnapshot({ product }: { product: ProductDto }) {
  return (
    <Descriptions size="small" column={1} className="panel">
      <Descriptions.Item label="当前库存">{qty(product.currentStock)}</Descriptions.Item>
      <Descriptions.Item label="平均进货价">{money(product.avgCost)}</Descriptions.Item>
      <Descriptions.Item label="库存价值">{money(product.stockValue)}</Descriptions.Item>
    </Descriptions>
  );
}

function productLabel(product: ProductDto) {
  return `${product.name} / ${product.category}${product.barcode ? ` / ${product.barcode}` : ''}`;
}

function statusTag(status: string) {
  return status === 'normal' ? <Tag color="green">正常</Tag> : <Tag color="red">已作废</Tag>;
}

function adjustmentTypeLabel(value: string) {
  return adjustmentTypes.find((item) => item.value === value)?.label ?? value;
}
