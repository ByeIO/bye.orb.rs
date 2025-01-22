#![allow(dead_code)] // 允许未使用的代码
#![allow(unused_variables)] // 允许未使用的变量
#![allow(unused_imports)] // 允许未使用的导入
#![allow(unused_mut)] // 允许未使用的可变变量
#![allow(unused_assignments)] // 允许未使用的赋值
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // 在非调试模式下设置Windows子系统
#![allow(rustdoc::missing_crate_level_docs)] // 允许缺少crate级别的文档
#![allow(unsafe_code)] // 允许使用unsafe代码
#![allow(clippy::undocumented_unsafe_blocks)] // 允许未文档化的unsafe块
#![allow(unused_must_use)] // 允许未使用的must_use结果
#![allow(non_snake_case)] // 允许非蛇形命名

use image::{ImageError, Rgba, GrayImage};
use imageproc::drawing::draw_line_segment_mut;
use cgmath::{prelude::{*}, Rad};

use crate::common;
use common::*;

// 常量
const DEFAULT_FAST_THRESHOLD:i32 = 50; // 默认的FAST阈值

/// 表示一个FAST关键点
#[derive(Debug, Clone, Copy)]
pub struct FastKeypoint {
    pub location: Point, // 关键点的位置
    pub score: i32, // 关键点的得分
    pub nms_dist: usize, // 非极大值抑制距离
    pub moment: Moment // 关键点的矩
}

impl Matchable for FastKeypoint {
    /// 计算两个FAST关键点之间的距离
    fn distance(&self, other: &FastKeypoint) -> usize {
        let ((ax, ay), (bx, by)) = (self.location, other.location);
        ((ax-bx).pow(2) as f32 + (ay-by).pow(2) as f32).sqrt() as usize
    }
}

/// 表示FAST算法的上下文
#[derive(Debug)]
pub struct FastContext {
    offsets: Vec<Point>, // 偏移量向量
    idx: Vec<usize>, // 索引向量
    cmp: Vec<i32>, // 比较向量
    radius: u32, // 半径
    n: u32 // 数量
}

#[allow(non_camel_case_types)]
/// 表示FAST算法的类型
#[derive(Debug, PartialEq)]
pub enum FastType {
    TYPE_7_12, // 类型7_12
    TYPE_9_16 // 类型9_16
}

impl FastType {
    /// 获取FAST算法的上下文
    pub fn get_context(&self) -> FastContext {
        match self {
            FastType::TYPE_7_12 => FastContext {
                offsets: vec![
                    ( 0, -2), ( 1, -2), ( 2, -1), ( 2,  0),
                    ( 2,  1), ( 1,  2), ( 0,  2), (-1,  2),
                    (-2,  1), (-2,  0), (-2, -1), (-1, -2)
                ],
                idx: vec![0, 6, 3, 9, 1, 2, 4, 5, 7, 8, 10, 11],
                cmp: vec![1, 1, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3],
                radius: 3,
                n: 9
            },
            FastType::TYPE_9_16 => FastContext {
                offsets: vec![
                    (0, -3), (1,  -3), (2, - 2), (3,  -1),
                    (3,  0), (3,   1), (2,   2), (1,   3),
                    (0,  3), (-1,  3), (-2,  2), (-3,  1),
                    (-3, 0), (-3, -1), (-2, -2), (-1, -3) 
                ],
                idx: vec![0, 8, 4, 12, 1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
                cmp: vec![1, 1, 1, 1, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
                radius: 4,
                n: 12
            }
        }
    }
}

/// 执行FAST算法以检测图像中的关键点
pub fn fast(img: &image::GrayImage, fast_type: Option<FastType>, threshold: Option<i32>) -> Result<Vec<FastKeypoint>, ImageError> {
    let threshold = threshold.unwrap_or(DEFAULT_FAST_THRESHOLD);
    let fast_type = fast_type.unwrap_or(FastType::TYPE_9_16);

    let ctx = fast_type.get_context();

    let mut fast_keypoint_matches = Vec::<FastKeypoint>::new();

    for y in ctx.radius .. img.height()-ctx.radius {
        'x_loop: for x in ctx.radius .. img.width()-ctx.radius {
            let center_pixel = img.get_pixel(x, y).0[0] as i32;
            let x = x as i32;
            let y = y as i32;
            let point:Point = (x, y);

            let mut score:i32 = 0;
            let mut similars:i32 = 0;

            for idx in 0..ctx.offsets.len() {
                let px_idx = ctx.idx[idx];
                let px = img.get_pixel((x + ctx.offsets[px_idx].0) as u32, (y + ctx.offsets[px_idx].1) as u32).0[0] as i32;
                let diff = (px - center_pixel).abs();
                
                if diff < threshold {
                    similars += 1;
                    if similars > ctx.cmp[idx] {
                        continue 'x_loop;
                    }
                } else {
                    score += diff;
                }
            }

            let moment = moment_centroid(img, &point, None);
            fast_keypoint_matches.push(FastKeypoint {
                location: point,
                score: score,
                nms_dist: 0,
                moment: moment
            });
        }
    }

    // 按得分排序
    fast_keypoint_matches.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(fast_keypoint_matches)
}

/// 表示图像的矩信息
#[derive(Debug, Clone, Copy)]
pub struct Moment {
    pub centroid: Point, // 质心
    pub moment: Point, // 矩
    pub rotation: f64 // 旋转角度
}