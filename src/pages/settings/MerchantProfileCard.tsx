import { Button, Card, Form, Input, Space, Typography } from 'antd';
import type { FormInstance } from 'antd/es/form';
import type { MerchantProfileDto } from '../../shared/types';

type MerchantProfileCardProps = {
  form: FormInstance<MerchantProfileDto>;
  merchant: MerchantProfileDto;
  onSave: () => void;
};

export function MerchantProfileCard({ form, merchant, onSave }: MerchantProfileCardProps) {
  return (
    <Form form={form} layout="vertical" className="dense-form">
      <Card title="商户信息">
        <Space.Compact block>
          <Form.Item label="商户名称" name="name" rules={[{ required: true, message: '请输入商户名称' }]} style={{ width: '25%' }}>
            <Input />
          </Form.Item>
          <Form.Item label="联系人" name="contact" style={{ width: '25%' }}>
            <Input />
          </Form.Item>
          <Form.Item label="电话" name="phone" style={{ width: '25%' }}>
            <Input />
          </Form.Item>
          <Form.Item label="地址" name="address" style={{ width: '25%' }}>
            <Input />
          </Form.Item>
        </Space.Compact>
        <Space.Compact block>
          <Form.Item label="Logo 路径" name="logoPath" style={{ width: '50%' }}>
            <Input placeholder="可选，例如 C:/Users/ww/Desktop/logo.png" />
          </Form.Item>
          <Form.Item label="备注" name="remark" style={{ width: '50%' }}>
            <Input />
          </Form.Item>
        </Space.Compact>
        <Space>
          <Button type="primary" onClick={onSave}>保存商户信息</Button>
          <Typography.Text type="secondary">当前单据抬头：{merchant.name || '我的商行'}</Typography.Text>
        </Space>
      </Card>
    </Form>
  );
}
