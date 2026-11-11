//! Export-only monitor cropping. Coordinates are supplied by the capture launcher.
use femtovg::{imgref::ImgVec, rgb::RGBA8};

pub fn crop(image: ImgVec<RGBA8>, baseline: &ImgVec<RGBA8>, layout: &str) -> ImgVec<RGBA8> {
    let monitors: Option<Vec<Vec<usize>>> = layout
        .split(';')
        .map(|rect| {
            let values: Option<Vec<usize>> = rect.split(',').map(|v| v.parse().ok()).collect();
            values.filter(|v| v.len() == 4 && v[2] > 0 && v[3] > 0)
        })
        .collect();
    let Some(monitors) = monitors.filter(|m| m.len() > 1) else {
        return image;
    };
    let (width, height) = (image.width(), image.height());
    if baseline.width() != width
        || baseline.height() != height
        || monitors.iter().any(|m| {
            m[0].checked_add(m[2]).is_none_or(|x| x > width)
                || m[1].checked_add(m[3]).is_none_or(|y| y > height)
        })
    {
        return image;
    }
    let mut changed = Vec::new();
    for (index, (pixel, original)) in image.buf().iter().zip(baseline.buf()).enumerate() {
        if pixel != original {
            changed.push((index % width, index / width));
        }
    }
    if changed.is_empty() {
        return image;
    }
    let candidates: Vec<_> = monitors
        .iter()
        .filter(|m| {
            changed
                .iter()
                .all(|&(x, y)| x >= m[0] && x < m[0] + m[2] && y >= m[1] && y < m[1] + m[3])
        })
        .collect();
    if candidates.len() != 1 {
        return image;
    }
    let m = candidates[0];
    let mut pixels = Vec::with_capacity(m[2] * m[3]);
    for y in m[1]..m[1] + m[3] {
        pixels.extend_from_slice(&image.buf()[y * width + m[0]..y * width + m[0] + m[2]]);
    }
    ImgVec::new(pixels, m[2], m[3])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crops_only_when_changes_fit_one_monitor() {
        let baseline = ImgVec::new(vec![RGBA8::new(0, 0, 0, 255); 32], 8, 4);
        let mut image = baseline.clone();
        image.buf_mut()[6] = RGBA8::new(255, 0, 0, 255);
        let layout = "0,0,4,4;4,0,4,4";
        let cropped = crop(image.clone(), &baseline, layout);
        assert_eq!((cropped.width(), cropped.height()), (4, 4));
        assert_eq!(cropped.buf()[2], image.buf()[6]);
        image.buf_mut()[1] = RGBA8::new(255, 0, 0, 255);
        assert_eq!(crop(image, &baseline, layout).width(), 8);
        assert_eq!(crop(baseline.clone(), &baseline, layout).width(), 8);
        assert_eq!(crop(baseline.clone(), &baseline, "broken").width(), 8);
    }
}
