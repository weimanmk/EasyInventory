import { Alert, Button, Card, Space } from 'antd';

type SetupGuideCardProps = {
  onReopen: () => void;
};

export function SetupGuideCard({ onReopen }: SetupGuideCardProps) {
  return (
    <Card title="初始化向导">
      <Space direction="vertical" style={{ width: '100%' }}>
        <Alert
          type="info"
          showIcon
          message="可重新打开初始化向导调整商户、行业模板、术语、功能开关和默认单据设置。该操作不会清空已有业务数据。"
        />
        <Button onClick={onReopen}>重新打开初始化向导</Button>
      </Space>
    </Card>
  );
}
