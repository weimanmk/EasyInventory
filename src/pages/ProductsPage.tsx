import { App, AutoComplete, Button, Drawer, Form, Input, InputNumber, Select, Space, Switch, Table, Tag, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { writeClientLog } from '../api/tauri';
import { money, qty, uniqueValues } from '../shared/format';
import type { ProductDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

export default function ProductsPage() {
  const { message, modal } = App.useApp();
  const [form] = Form.useForm();
  const { products, setProducts, productFilter, setProductFilter } = useAppStore();
  const [category, setCategory] = useState<string>();
  const [keyword, setKeyword] = useState('');
  const [onlyLowStock, setOnlyLowStock] = useState(false);
  const [editing, setEditing] = useState<ProductDto | null>(null);
  const categories = useMemo(() => uniqueValues(products, (item) => item.category), [products]);
  const categoryOptions = useMemo(() => categories.map((item) => ({ value: item, label: item })), [categories]);
  const filtered = products.filter((item) => {
    const matchCategory = !category || item.category === category;
    const matchKeyword = !keyword || item.name.includes(keyword) || item.barcode?.includes(keyword);
    const matchLow = !onlyLowStock || item.currentStock <= item.safetyStock;
    return matchCategory && matchKeyword && matchLow;
  });

  async function refresh() {
    const nextProducts = await api.products({ isActive: true });
    setProducts(nextProducts);
    return nextProducts;
  }

  useEffect(() => {
    if (productFilter?.onlyLowStock) {
      setOnlyLowStock(true);
      setProductFilter({});
    }
  }, [productFilter, setProductFilter]);

  async function save() {
    void writeClientLog('INFO', 'products', '点击商品保存', { editingId: editing?.id ?? null });
    let values: Record<string, unknown>;
    try {
      values = await form.validateFields();
    } catch (error) {
      void writeClientLog('WARN', 'products', '商品表单校验未通过', error);
      return;
    }
    try {
      let saved: ProductDto;
      if (editing?.id) {
        void writeClientLog('INFO', 'products', '提交更新商品', { id: editing.id, values });
        saved = await api.updateProduct(editing.id, values);
      } else {
        void writeClientLog('INFO', 'products', '提交新增商品', { values });
        saved = await api.createProduct(values);
      }
      const nextProducts = await refresh();
      const visibleProduct = nextProducts.find((item) => item.id === saved.id) ?? saved;
      void writeClientLog('INFO', 'products', '商品保存后刷新完成', {
        savedId: saved.id,
        savedName: saved.name,
        savedCategory: saved.category,
        refreshedCount: nextProducts.length,
        visibleAfterRefresh: nextProducts.some((item) => item.id === saved.id)
      });
      setCategory(saved.category);
      setKeyword('');
      setOnlyLowStock(false);
      setEditing(null);
      form.resetFields();
      message.success(`保存成功：${visibleProduct.name}`);
    } catch (error) {
      void writeClientLog('ERROR', 'products', '商品保存失败', error);
      message.error(error instanceof Error ? error.message : '保存失败');
    }
  }

  function openNewProduct() {
    form.resetFields();
    setEditing({} as ProductDto);
  }

  function closeEditor() {
    setEditing(null);
    form.resetFields();
  }

  async function disable(row: ProductDto) {
    try {
      await api.disableProduct(row.id);
      message.success('商品已删除');
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除失败');
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <Typography.Title level={2}>商品库存</Typography.Title>
        <Button type="primary" onClick={openNewProduct}>新增商品</Button>
      </div>
      <div className="toolbar panel">
        <Select allowClear placeholder="类别" value={category} style={{ width: 160 }} options={categoryOptions} onChange={setCategory} />
        <Input allowClear placeholder="搜索商品名或条码" value={keyword} onChange={(event) => setKeyword(event.target.value)} style={{ width: 260 }} />
        <Space>低库存 <Switch checked={onlyLowStock} onChange={setOnlyLowStock} /></Space>
        <Button onClick={() => void refresh()}>刷新</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={filtered}
        rowClassName={(row) => row.currentStock < 0 ? 'negative-row' : row.currentStock <= row.safetyStock ? 'warning-row' : ''}
        columns={[
          { title: '商品名称', dataIndex: 'name' },
          { title: '类别', dataIndex: 'category', width: 110 },
          { title: '条码', dataIndex: 'barcode', width: 150 },
          { title: '当前库存', render: (_, row) => qty(row.currentStock), align: 'right', width: 110 },
          { title: '安全库存', dataIndex: 'safetyStock', align: 'right', width: 110 },
          { title: '平均进货价', render: (_, row) => money(row.avgCost), align: 'right', width: 120 },
          { title: '默认售价', render: (_, row) => money(row.defaultPrice), align: 'right', width: 110 },
          { title: '库存价值', render: (_, row) => money(row.stockValue), align: 'right', width: 120 },
          { title: '状态', render: (_, row) => <Tag color={row.isActive ? 'green' : 'default'}>{row.isActive ? '启用' : '停用'}</Tag>, width: 90 },
          {
            title: '操作',
            render: (_, row) => (
              <Space>
                <Button size="small" onClick={() => { form.resetFields(); setEditing(row); form.setFieldsValue(row); }}>编辑</Button>
                <Button
                  size="small"
                  danger
                  onClick={() => modal.confirm({
                    title: '删除该商品？',
                    content: '商品会被停用并从常用列表隐藏，历史订单和库存流水不会被破坏。',
                    okText: '删除',
                    okButtonProps: { danger: true },
                    onOk: () => disable(row)
                  })}
                >
                  删除
                </Button>
              </Space>
            ),
            width: 150
          }
        ]}
      />
      <Drawer title={editing?.id ? '编辑商品' : '新增商品'} open={!!editing} onClose={closeEditor} width={420}>
        <Form form={form} layout="vertical" className="dense-form">
          <Form.Item label="商品名称" name="name" rules={[{ required: true, message: '请输入商品名称' }]}><Input /></Form.Item>
          <Form.Item label="类别" name="category" rules={[{ required: true, message: '请输入类别' }]}>
            <AutoComplete
              options={categoryOptions}
              placeholder="选择已有类别或输入新类别"
              filterOption={(inputValue, option) => String(option?.value ?? '').toLowerCase().includes(inputValue.toLowerCase())}
            />
          </Form.Item>
          <Form.Item label="条码" name="barcode"><Input /></Form.Item>
          <Form.Item label="默认售价" name="defaultPrice"><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          <Form.Item label="安全库存" name="safetyStock"><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          <Form.Item label="单位" name="unit"><Input /></Form.Item>
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" block onClick={() => void save()}>保存</Button>
        </Form>
      </Drawer>
    </div>
  );
}
