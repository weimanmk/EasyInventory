import { Button, Card, Form, Input, InputNumber, Select, Space, Switch } from 'antd';
import type { FormInstance } from 'antd/es/form';
import type { DocumentTemplateDto, MerchantProfileDto, TermSettingsDto } from '../../shared/types';

type DocumentTemplateSettingsCardProps = {
  form: FormInstance;
  documentTemplates: DocumentTemplateDto[];
  merchant: MerchantProfileDto;
  terms: TermSettingsDto;
  printers: string[];
  templateValues: DocumentTemplateFormValues;
  onSave: () => void;
  onApplyTemplate: (templateId: string) => void;
  onOpenExportsFolder: () => void;
  onOpenLogsFolder: () => void;
};

type DocumentTemplateFormValues = {
  templateStoreName?: string;
  templateFooterText?: string;
  templateShowBarcode?: boolean;
  templateProductLabel?: string;
  templateQuantityLabel?: string;
  templatePriceLabel?: string;
  templateAmountLabel?: string;
  templateRemarkLabel?: string;
  templateOrientation?: string;
};

export function DocumentTemplateSettingsCard({
  form,
  documentTemplates,
  merchant,
  terms,
  printers,
  templateValues,
  onSave,
  onApplyTemplate,
  onOpenExportsFolder,
  onOpenLogsFolder
}: DocumentTemplateSettingsCardProps) {
  return (
    <Form form={form} layout="vertical" className="dense-form">
      <Card title="系统与单据模板设置">
        <Space.Compact block>
          <Form.Item label="每日自动备份" name="dailyAutoBackup" valuePropName="checked" style={{ width: '25%' }}>
            <Switch />
          </Form.Item>
          <Form.Item label="默认打印模板" name="defaultPrintTemplate" style={{ width: '25%' }}>
            <Select options={documentTemplates.map((item) => ({ value: item.id, label: item.name }))} />
          </Form.Item>
          <Form.Item label="默认导出格式" name="defaultExportFormat" style={{ width: '25%' }}>
            <Select options={[{ value: 'xlsx', label: 'Excel xlsx' }]} />
          </Form.Item>
          <Form.Item label="默认打印机" name="defaultPrinter" style={{ width: '25%' }}>
            <Select allowClear options={printers.map((item) => ({ value: item, label: item }))} />
          </Form.Item>
        </Space.Compact>
        <Space.Compact block>
          <Form.Item label="店名" name="templateStoreName" style={{ width: '25%' }}>
            <Input />
          </Form.Item>
          <Form.Item label="页脚/默认备注" name="templateFooterText" style={{ width: '25%' }}>
            <Input />
          </Form.Item>
          <Form.Item label="纸张方向" name="templateOrientation" style={{ width: '20%' }}>
            <Select options={[{ value: 'landscape', label: '横向' }]} />
          </Form.Item>
          <Form.Item label="页边距" name="templateMargin" style={{ width: '15%' }}>
            <InputNumber min={0} max={2} step={0.1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label="显示条码" name="templateShowBarcode" valuePropName="checked" style={{ width: '15%' }}>
            <Switch />
          </Form.Item>
        </Space.Compact>
        <Space.Compact block>
          <Form.Item label="商品列名" name="templateProductLabel" style={{ width: '20%' }}><Input /></Form.Item>
          <Form.Item label="数量列名" name="templateQuantityLabel" style={{ width: '20%' }}><Input /></Form.Item>
          <Form.Item label="价格列名" name="templatePriceLabel" style={{ width: '20%' }}><Input /></Form.Item>
          <Form.Item label="金额列名" name="templateAmountLabel" style={{ width: '20%' }}><Input /></Form.Item>
          <Form.Item label="备注列名" name="templateRemarkLabel" style={{ width: '20%' }}><Input /></Form.Item>
        </Space.Compact>
        <div className="template-preview">
          <div className="template-preview-title">{templateValues.templateStoreName || merchant.name || '我的商行'}</div>
          <div className="template-preview-meta">
            {terms.customer}：测试{terms.customer}　单号：20260601001　方向：{templateValues.templateOrientation === 'landscape' ? '横向' : '纵向'}
          </div>
          <div className="template-preview-row">
            <span>{templateValues.templateShowBarcode === false ? '' : '条码'}</span>
            <span>{templateValues.templateProductLabel || '商品名称'}</span>
            <span>{templateValues.templateQuantityLabel || '数量'}</span>
            <span>{templateValues.templatePriceLabel || '价格'}</span>
            <span>{templateValues.templateAmountLabel || '总价格'}</span>
            <span>{templateValues.templateRemarkLabel || '备注'}</span>
          </div>
          <div className="template-preview-footer">{templateValues.templateFooterText || '页脚/默认备注'}</div>
        </div>
        <Space>
          <Button type="primary" onClick={onSave}>保存设置</Button>
          <Button
            onClick={() => form.setFieldsValue({
              templateStoreName: merchant.name || '我的商行',
              templateFooterText: '',
              templateShowBarcode: true,
              templateProductLabel: '商品名称',
              templateQuantityLabel: '数量',
              templatePriceLabel: '价格',
              templateAmountLabel: '总价格',
              templateRemarkLabel: '备注',
              templateOrientation: 'landscape',
              templateMargin: 0
            })}
          >
            恢复默认模板
          </Button>
          {documentTemplates.map((item) => (
            <Button key={item.id} onClick={() => onApplyTemplate(item.id)}>{item.name}</Button>
          ))}
          <Button onClick={onOpenExportsFolder}>打开导出目录</Button>
          <Button onClick={onOpenLogsFolder}>打开日志目录</Button>
        </Space>
      </Card>
    </Form>
  );
}
