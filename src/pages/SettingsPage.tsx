import { App, Button, Card, Descriptions, Form, Input, List, Select, Space, Switch, Typography } from 'antd';
import { useEffect, useState } from 'react';
import { api } from '../api/inventory';
import type { AppStatusDto, BackupDto, ImportResult, SettingDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

const defaultExcelPath = 'C:/Users/ww/Desktop/work/订单库存表3.02 - 副本 (2).xlsm';

export default function SettingsPage() {
  const { message } = App.useApp();
  const { status, setStatus, setProducts, setCustomers } = useAppStore();
  const [form] = Form.useForm();
  const [appStatus, setAppStatus] = useState<AppStatusDto | undefined>(status);
  const [excelPath, setExcelPath] = useState(defaultExcelPath);
  const [importResult, setImportResult] = useState<ImportResult | null>();
  const [backups, setBackups] = useState<BackupDto[]>([]);
  const [printers, setPrinters] = useState<string[]>([]);

  async function refresh() {
    const [nextStatus, nextBackups, settings, nextPrinters] = await Promise.all([
      api.status(),
      api.backups(),
      api.settings(),
      api.printers().catch(() => [] as string[])
    ]);
    setAppStatus(nextStatus);
    setStatus(nextStatus);
    setBackups(nextBackups);
    setPrinters(nextPrinters);
    form.setFieldsValue(settingsToForm(settings));
  }

  async function importExcel() {
    try {
      const result = await api.importExcel(excelPath);
      setImportResult(result);
      setProducts(await api.products({ isActive: true }));
      setCustomers(await api.customers({ isActive: true }));
      message.success(`导入完成：商品 ${result.productCount}，客户 ${result.customerCount}，流水 ${result.movementCount}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导入失败');
    }
  }

  async function backup() {
    try {
      const path = await api.backup();
      message.success(`备份完成：${path}`);
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '备份失败');
    }
  }

  async function saveSettings() {
    try {
      await api.saveSettings(form.getFieldsValue());
      message.success('设置已保存');
      await refresh();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '设置保存失败');
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  return (
    <div className="page">
      <div className="page-title"><Typography.Title level={2}>系统设置 / 备份</Typography.Title></div>
      <Card title="本地路径">
        <Descriptions column={1} size="small">
          <Descriptions.Item label="数据库">{appStatus?.databasePath}</Descriptions.Item>
          <Descriptions.Item label="数据目录">{appStatus?.dataDir}</Descriptions.Item>
          <Descriptions.Item label="订单目录">{appStatus?.ordersDir}</Descriptions.Item>
          <Descriptions.Item label="导出目录">{appStatus?.exportsDir}</Descriptions.Item>
          <Descriptions.Item label="备份目录">{appStatus?.backupsDir}</Descriptions.Item>
          <Descriptions.Item label="日志目录">{appStatus?.logsDir}</Descriptions.Item>
          <Descriptions.Item label="版本">{appStatus?.version}</Descriptions.Item>
        </Descriptions>
      </Card>
      <Card title="系统设置">
        <Form form={form} layout="vertical" className="dense-form">
          <Space.Compact block>
            <Form.Item label="每日自动备份" name="dailyAutoBackup" valuePropName="checked" style={{ width: '25%' }}>
              <Switch />
            </Form.Item>
            <Form.Item label="默认打印模板" name="defaultPrintTemplate" style={{ width: '25%' }}>
              <Select options={[{ value: 'excel', label: 'Excel 打印区' }]} />
            </Form.Item>
            <Form.Item label="默认导出格式" name="defaultExportFormat" style={{ width: '25%' }}>
              <Select options={[{ value: 'xlsx', label: 'Excel xlsx' }]} />
            </Form.Item>
            <Form.Item label="默认打印机" name="defaultPrinter" style={{ width: '25%' }}>
              <Select allowClear options={printers.map((item) => ({ value: item, label: item }))} />
            </Form.Item>
          </Space.Compact>
          <Space>
            <Button type="primary" onClick={() => void saveSettings()}>保存设置</Button>
            <Button onClick={() => void api.openExportsFolder()}>打开导出目录</Button>
            <Button onClick={() => void api.openLogsFolder()}>打开日志目录</Button>
          </Space>
        </Form>
      </Card>
      <Card title="Excel 一次性迁移">
        <Space.Compact style={{ width: '100%' }}>
          <Input value={excelPath} onChange={(event) => setExcelPath(event.target.value)} />
          <Button type="primary" onClick={() => void importExcel()}>导入</Button>
        </Space.Compact>
        {importResult && (
          <List
            size="small"
            header={`商品 ${importResult.productCount} / 客户 ${importResult.customerCount} / 流水 ${importResult.movementCount}`}
            dataSource={[...importResult.warnings, ...importResult.errors]}
            renderItem={(item) => <List.Item>{item}</List.Item>}
          />
        )}
      </Card>
      <Card title="备份">
        <Space>
          <Button onClick={() => void backup()}>立即备份</Button>
          <Button onClick={() => void api.openBackupFolder()}>打开备份目录</Button>
        </Space>
        <List
          size="small"
          dataSource={backups}
          renderItem={(item) => <List.Item>{item.createdAt} · {item.backupType} · {item.status} · {item.backupPath}</List.Item>}
        />
      </Card>
    </div>
  );
}

function settingsToForm(settings: SettingDto[]) {
  const map = Object.fromEntries(settings.map((item) => [item.key, item.value]));
  return {
    dailyAutoBackup: map.daily_auto_backup !== 'false',
    defaultPrintTemplate: map.default_print_template || 'excel',
    defaultExportFormat: map.default_export_format || 'xlsx',
    defaultPrinter: map.default_printer || undefined
  };
}
