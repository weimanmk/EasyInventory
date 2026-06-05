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
  const [batchForm] = Form.useForm();
  const { products, setProducts, productFilter, setProductFilter, terms } = useAppStore();
  const [category, setCategory] = useState<string>();
  const [keyword, setKeyword] = useState('');
  const [onlyLowStock, setOnlyLowStock] = useState(false);
  const [editing, setEditing] = useState<ProductDto | null>(null);
  const [batchOpen, setBatchOpen] = useState(false);
  const [selectedProductIds, setSelectedProductIds] = useState<number[]>([]);
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
    void writeClientLog('INFO', 'products', `点击${terms.product}保存`, { editingId: editing?.id ?? null });
    let values: Record<string, unknown>;
    try {
      values = await form.validateFields();
    } catch (error) {
      void writeClientLog('WARN', 'products', `${terms.product}表单校验未通过`, error);
      return;
    }
    try {
      let saved: ProductDto;
      if (editing?.id) {
        void writeClientLog('INFO', 'products', `提交更新${terms.product}`, { id: editing.id, values });
        saved = await api.updateProduct(editing.id, values);
      } else {
        void writeClientLog('INFO', 'products', `提交新增${terms.product}`, { values });
        saved = await api.createProduct(values);
      }
      const nextProducts = await refresh();
      const visibleProduct = nextProducts.find((item) => item.id === saved.id) ?? saved;
      void writeClientLog('INFO', 'products', `${terms.product}保存后刷新完成`, {
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
      void writeClientLog('ERROR', 'products', `${terms.product}保存失败`, error);
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

  function openBatchEditor() {
    if (selectedProductIds.length === 0) {
      message.warning(`请选择${terms.product}`);
      return;
    }
    batchForm.resetFields();
    setBatchOpen(true);
  }

  async function saveBatch() {
    const values = await batchForm.validateFields();
    try {
      const result = await api.batchUpdateProducts({
        ids: selectedProductIds,
        ...values
      });
      await refresh();
      setSelectedProductIds([]);
      setBatchOpen(false);
      message.success(`已批量更新 ${result.affectedCount} 个${terms.product}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '批量编辑失败');
    }
  }

  async function disable(row: ProductDto) {
    try {
      await api.disableProduct(row.id);
      message.success(`${terms.product}已删除`);
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除失败');
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <Typography.Title level={2}>{terms.product}库存</Typography.Title>
        <Space>
          <Button disabled={selectedProductIds.length === 0} onClick={openBatchEditor}>批量编辑</Button>
          <Button type="primary" onClick={openNewProduct}>新增{terms.product}</Button>
        </Space>
      </div>
      <div className="toolbar panel">
        <Select allowClear placeholder={terms.category} value={category} style={{ width: 160 }} options={categoryOptions} onChange={setCategory} />
        <Input allowClear placeholder={`搜索${terms.product}名或条码`} value={keyword} onChange={(event) => setKeyword(event.target.value)} style={{ width: 260 }} />
        <Space>低库存 <Switch checked={onlyLowStock} onChange={setOnlyLowStock} /></Space>
        <Button onClick={() => void refresh()}>刷新</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={filtered}
        rowSelection={{
          selectedRowKeys: selectedProductIds,
          onChange: (keys) => setSelectedProductIds(keys as number[])
        }}
        rowClassName={(row) => row.currentStock < 0 ? 'negative-row' : row.currentStock <= row.safetyStock ? 'warning-row' : ''}
        columns={[
          { title: `${terms.product}名称`, dataIndex: 'name' },
          { title: terms.category, dataIndex: 'category', width: 110 },
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
                    title: `删除该${terms.product}？`,
                    content: `${terms.product}会被停用并从常用列表隐藏，历史订单和库存流水不会被破坏。`,
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
      <Drawer title={editing?.id ? `编辑${terms.product}` : `新增${terms.product}`} open={!!editing} onClose={closeEditor} width={420}>
        <Form form={form} layout="vertical" className="dense-form">
          <Form.Item label={`${terms.product}名称`} name="name" rules={[{ required: true, message: `请输入${terms.product}名称` }]}><Input /></Form.Item>
          <Form.Item label={terms.category} name="category" rules={[{ required: true, message: `请输入${terms.category}` }]}>
            <AutoComplete
              options={categoryOptions}
              placeholder={`选择已有${terms.category}或输入新${terms.category}`}
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
      <Drawer title={`批量编辑${terms.product}（${selectedProductIds.length}）`} open={batchOpen} onClose={() => setBatchOpen(false)} width={420}>
        <Form form={batchForm} layout="vertical" className="dense-form">
          <Form.Item label={terms.category} name="category">
            <AutoComplete
              options={categoryOptions}
              placeholder="留空则不修改"
              filterOption={(inputValue, option) => String(option?.value ?? '').toLowerCase().includes(inputValue.toLowerCase())}
            />
          </Form.Item>
          <Form.Item label="默认售价" name="defaultPrice"><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          <Form.Item label="安全库存" name="safetyStock"><InputNumber min={0} style={{ width: '100%' }} /></Form.Item>
          <Form.Item label="单位" name="unit"><Input /></Form.Item>
          <Form.Item label="状态" name="isActive">
            <Select
              allowClear
              placeholder="留空则不修改"
              options={[
                { value: true, label: '启用' },
                { value: false, label: '停用' }
              ]}
            />
          </Form.Item>
          <Button type="primary" block onClick={() => void saveBatch()}>保存批量修改</Button>
        </Form>
      </Drawer>
    </div>
  );
}
