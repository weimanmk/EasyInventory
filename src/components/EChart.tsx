import * as echarts from 'echarts';
import type { ECharts, EChartsOption } from 'echarts';
import { useEffect, useRef } from 'react';

type EChartProps = {
  option: EChartsOption;
  height?: number;
  empty?: boolean;
  emptyText?: string;
};

export default function EChart({ option, height = 320, empty = false, emptyText = '暂无统计数据' }: EChartProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<ECharts | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return undefined;
    }
    const chart = echarts.init(container);
    chartRef.current = chart;

    const resize = () => chart.resize();
    const observer = new ResizeObserver(resize);
    observer.observe(container);
    window.addEventListener('resize', resize);

    return () => {
      observer.disconnect();
      window.removeEventListener('resize', resize);
      chart.dispose();
      chartRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!chartRef.current) {
      return;
    }
    if (empty) {
      chartRef.current.clear();
      return;
    }
    chartRef.current.setOption(option, true);
  }, [empty, option]);

  return (
    <div className="echart-box" style={{ height }}>
      <div ref={containerRef} className="echart-canvas" />
      {empty ? <div className="echart-empty">{emptyText}</div> : null}
    </div>
  );
}
