import { App, Button, Drawer, Form, Input, Space, Table, Tag, Typography } from 'antd';
import { useEffect, useState } from 'react';
import { api } from '../api/inventory';
import type { SupplierDto } from '../shared/types';

export default function SuppliersPage() {
  const { message, modal } = App.useApp();
  const [form] = Form.useForm();
  const [batchForm] = Form.useForm();
  const [suppliers, setSuppliers] = useState<SupplierDto[]>([]);
  const [keyword, setKeyword] = useState('');
  const [editing, setEditing] = useState<SupplierDto | null>(null);
  const [batchOpen, setBatchOpen] = useState(false);
  const [selectedSupplierIds, setSelectedSupplierIds] = useState<number[]>([]);

  const filtered = suppliers.filter((item) => {
    const text = `${item.name}${item.contact ?? ''}${item.phone ?? ''}${item.address ?? ''}`;
    return !keyword || text.includes(keyword);
  });

  async function refresh() {
    const rows = await api.suppliers({ isActive: true });
    setSuppliers(rows);
    return rows;
  }

  useEffect(() => {
    void refresh().catch((error) => {
      message.warning(error instanceof Error ? error.message : '供应商加载失败');
    });
  }, [message]);

  function openNewSupplier() {
    form.resetFields();
    setEditing({} as SupplierDto);
  }

  function closeEditor() {
    setEditing(null);
    form.resetFields();
  }

  function openBatchEditor() {
    if (selectedSupplierIds.length === 0) {
      message.warning('请选择供应商');
      return;
    }
    batchForm.resetFields();
    setBatchOpen(true);
  }

  async function saveBatch() {
    const values = await batchForm.validateFields();
    try {
      const result = await api.batchUpdateSuppliers({
        ids: selectedSupplierIds,
        ...values
      });
      await refresh();
      setSelectedSupplierIds([]);
      setBatchOpen(false);
      message.success(`已批量更新 ${result.affectedCount} 个供应商`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '批量编辑失败');
    }
  }

  async function save() {
    const values = await form.validateFields();
    try {
      const saved = editing?.id
        ? await api.updateSupplier(editing.id, values)
        : await api.createSupplier(values);
      await refresh();
      setKeyword('');
      closeEditor();
      message.success(`保存成功：${saved.name}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '保存失败');
    }
  }

  async function disable(row: SupplierDto) {
    try {
      await api.disableSupplier(row.id);
      await refresh();
      message.success('供应商已删除');
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除失败');
    }
  }

  return (
    <div className="page">
      <div className="page-title">
        <div>
          <Typography.Title level={2}>供应商管理</Typography.Title>
          <Typography.Text type="secondary">维护入库可选供应商资料</Typography.Text>
        </div>
        <Space>
          <Button disabled={selectedSupplierIds.length === 0} onClick={openBatchEditor}>批量编辑</Button>
          <Button type="primary" onClick={openNewSupplier}>新增供应商</Button>
        </Space>
      </div>
      <div className="toolbar panel">
        <Input
          allowClear
          placeholder="搜索供应商、联系人、电话或地址"
          value={keyword}
          onChange={(event) => setKeyword(event.target.value)}
          style={{ width: 320 }}
        />
        <Button onClick={() => void refresh()}>刷新</Button>
      </div>
      <Table
        rowKey="id"
        dataSource={filtered}
        rowSelection={{
          selectedRowKeys: selectedSupplierIds,
          onChange: (keys) => setSelectedSupplierIds(keys as number[])
        }}
        columns={[
          { title: '供应商名称', dataIndex: 'name' },
          { title: '联系人', dataIndex: 'contact', width: 120 },
          { title: '电话', dataIndex: 'phone', width: 150 },
          { title: '地址', dataIndex: 'address' },
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
                    title: '删除该供应商？',
                    content: '供应商会被停用并从入库选项隐藏，历史入库记录不会被破坏。',
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
      <Drawer title={editing?.id ? '编辑供应商' : '新增供应商'} open={!!editing} onClose={closeEditor} width={420}>
        <Form form={form} layout="vertical" className="dense-form">
          <Form.Item label="供应商名称" name="name" rules={[{ required: true, message: '请输入供应商名称' }]}>
            <Input />
          </Form.Item>
          <Form.Item label="联系人" name="contact"><Input /></Form.Item>
          <Form.Item label="电话" name="phone"><Input /></Form.Item>
          <Form.Item label="地址" name="address"><Input /></Form.Item>
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" block onClick={() => void save()}>保存</Button>
        </Form>
      </Drawer>
      <Drawer title={`批量编辑供应商（${selectedSupplierIds.length}）`} open={batchOpen} onClose={() => setBatchOpen(false)} width={420}>
        <Form form={batchForm} layout="vertical" className="dense-form">
          <Form.Item label="联系人" name="contact"><Input /></Form.Item>
          <Form.Item label="电话" name="phone"><Input /></Form.Item>
          <Form.Item label="地址" name="address"><Input /></Form.Item>
          <Form.Item label="备注" name="remark"><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" block onClick={() => void saveBatch()}>保存批量修改</Button>
        </Form>
      </Drawer>
    </div>
  );
}
