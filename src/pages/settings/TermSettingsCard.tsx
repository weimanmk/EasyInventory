import { Button, Card, Form, Input, Space } from 'antd';
import type { FormInstance } from 'antd/es/form';
import type { TermSettingsDto } from '../../shared/types';

type TermSettingsCardProps = {
  form: FormInstance<TermSettingsDto>;
  onSave: () => void;
};

export function TermSettingsCard({ form, onSave }: TermSettingsCardProps) {
  return (
    <Form form={form} layout="vertical" className="dense-form">
      <Card title="业务术语">
        <Space.Compact block>
          <Form.Item label="客户显示名" name="customer" style={{ width: '25%' }}><Input /></Form.Item>
          <Form.Item label="地区显示名" name="region" style={{ width: '25%' }}><Input /></Form.Item>
          <Form.Item label="商品显示名" name="product" style={{ width: '25%' }}><Input /></Form.Item>
          <Form.Item label="类别显示名" name="category" style={{ width: '25%' }}><Input /></Form.Item>
        </Space.Compact>
        <Space.Compact block>
          <Form.Item label="规则显示名" name="rule" style={{ width: '33.33%' }}><Input /></Form.Item>
          <Form.Item label="额度显示名" name="credit" style={{ width: '33.33%' }}><Input /></Form.Item>
          <Form.Item label="默认客户显示名" name="guestCustomer" style={{ width: '33.33%' }}><Input /></Form.Item>
        </Space.Compact>
        <Button type="primary" onClick={onSave}>保存术语配置</Button>
      </Card>
    </Form>
  );
}
