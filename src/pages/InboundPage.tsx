import { App, Button, Card, DatePicker, Form, Input, InputNumber, Select, Space, Table, Typography } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { money, uniqueValues } from '../shared/format';
import type { InboundRecordDto, SupplierDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function InboundPage() {
  const { message } = App.useApp();
  const [form] = Form.useForm();
  const { products, setProducts } = useAppStore();
  const [category, setCategory] = useState<string>();
  const [records, setRecords] = useState<InboundRecordDto[]>([]);
  const [suppliers, setSuppliers] = useState<SupplierDto[]>([]);
  const categories = useMemo(() => uniqueValues(products, (item) => item.category), [products]);
  const filteredProducts = useMemo(
    () => products.filter((item) => !category || item.category === category),
    [category, products]
  );

  async function loadRecords() {
    const data = await api.inboundRecords({});
    setRecords(data);
  }

  useEffect(() => {
    void api.suppliers({ isActive: true }).then(setSuppliers).catch(() => setSuppliers([]));
    void loadRecords();
  }, []);

  async function save() {
    const values = await form.validateFields();
    try {
      await api.createInbound({
        inboundDate: values.inboundDate.format('YYYY-MM-DD'),
        productId: values.productId,
        supplierId: values.supplierId,
        quantity: values.quantity,
        unitCost: values.unitCost,
        remark: values.remark
      });
      message.success('入库成功');
      form.setFieldsValue({ quantity: undefined, remark: undefined });
      setProducts(await api.products({ isActive: true }));
      await loadRecords();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '入库失败');
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>入库</Typography.Title>
          <Typography.Text type="secondary">录入进货数量与进货价，自动更新库存均价</Typography.Text>
        </div>
        <Button onClick={() => void loadRecords()}>刷新记录</Button>
      </div>
      <div className="two-column">
        <Card title="入库表单">
          <Form form={form} layout="vertical" initialValues={{ inboundDate: dayjs() }} className="dense-form">
            <Form.Item label="日期" name="inboundDate" rules={[{ required: true }]}>
              <DatePicker style={{ width: '100%' }} />
            </Form.Item>
            <Space.Compact block>
              <Form.Item label="类别" style={{ width: '40%' }}>
                <Select options={categories.map((item) => ({ value: item, label: item }))} onChange={setCategory} />
              </Form.Item>
              <Form.Item label="商品" name="productId" rules={[{ required: true }]} style={{ width: '60%' }}>
                <Select
                  showSearch
                  optionFilterProp="label"
                  options={filteredProducts.map((item) => ({ value: item.id, label: item.name }))}
                />
              </Form.Item>
            </Space.Compact>
            <Form.Item label="供应商" name="supplierId">
              <Select
                allowClear
                showSearch
                optionFilterProp="label"
                options={suppliers.map((item) => ({ value: item.id, label: item.name }))}
              />
            </Form.Item>
            <Space.Compact block>
              <Form.Item label="数量" name="quantity" rules={[{ required: true }]} style={{ width: '50%' }}>
                <InputNumber min={0.01} style={{ width: '100%' }} />
              </Form.Item>
              <Form.Item label="进货价" name="unitCost" rules={[{ required: true }]} style={{ width: '50%' }}>
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Space.Compact>
            <Form.Item label="备注" name="remark"><Input /></Form.Item>
            <Button type="primary" block onClick={() => void save()}>保存入库</Button>
          </Form>
        </Card>
        <Card title="最近入库记录">
          <Table
            rowKey="id"
            size="small"
            dataSource={records}
            columns={[
              { title: '日期', dataIndex: 'inboundDate', width: 110 },
              { title: '商品', dataIndex: 'productName' },
              { title: '供应商', dataIndex: 'supplierName', width: 120 },
              { title: '数量', dataIndex: 'quantity', align: 'right', width: 90 },
              { title: '进货价', render: (_, row) => money(row.unitCost), align: 'right', width: 100 },
              { title: '金额', render: (_, row) => money(row.amount), align: 'right', width: 100 }
            ]}
          />
        </Card>
      </div>
    </div>
  );
}
