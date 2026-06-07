import { Button, Card, Form, Switch, Table } from 'antd';
import type { FormInstance } from 'antd/es/form';
import type { FeatureFlagsDto, IndustryTemplateDto } from '../../shared/types';

type IndustryFeatureCardProps = {
  form: FormInstance<FeatureFlagsDto>;
  industryTemplates: IndustryTemplateDto[];
  onApplyTemplate: (template: IndustryTemplateDto) => void;
  onSaveFeatures: () => void;
};

const featureItems: Array<{ name: keyof FeatureFlagsDto; label: string }> = [
  { name: 'supplierLedger', label: '供应商采购台账' },
  { name: 'customerRules', label: '价格规则' },
  { name: 'monthlyCredit', label: '返利额度' },
  { name: 'receivables', label: '欠款收款' },
  { name: 'productRanking', label: '商品经营排行' },
  { name: 'customerAnalysis', label: '客户经营分析' },
  { name: 'inventoryControl', label: '库存盘点' },
  { name: 'diagnostics', label: '诊断中心' }
];

export function IndustryFeatureCard({
  form,
  industryTemplates,
  onApplyTemplate,
  onSaveFeatures
}: IndustryFeatureCardProps) {
  return (
    <Card title="行业模板与功能开关">
      <Table
        rowKey="id"
        size="small"
        dataSource={industryTemplates}
        pagination={false}
        columns={[
          { title: '模板', dataIndex: 'name', width: 140 },
          { title: '说明', dataIndex: 'description' },
          { title: '默认单据', dataIndex: 'orderTemplate', width: 120 },
          {
            title: '操作',
            width: 100,
            render: (_, row) => <Button size="small" onClick={() => onApplyTemplate(row)}>应用</Button>
          }
        ]}
      />
      <Form form={form} layout="vertical" className="dense-form" style={{ marginTop: 12 }}>
        <div className="feature-grid">
          {featureItems.map((item) => (
            <Form.Item key={item.name} label={item.label} name={item.name} valuePropName="checked">
              <Switch />
            </Form.Item>
          ))}
        </div>
        <Button onClick={onSaveFeatures}>保存功能开关</Button>
      </Form>
    </Card>
  );
}
