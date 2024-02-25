
use std::collections::{HashSet, HashMap};
use crate::utils::rc_wrapper::NodeRef;
use log::debug;

pub fn draw_graph(nodes: &HashSet<NodeRef>, head: &NodeRef, title: &str){//, node_size: usize){
    debug!("Plotting graph");
    let NODE_SIZE: f32 = 10_f32;
    let X_OFFSET: f32 = 3_f32*NODE_SIZE;
    let TREE_DEPTH_OFFSET: u32 = 100;

    // Gather min depth
    let y_min = nodes.iter().map(|x| (*x).borrow().depth).min().unwrap();
    let y_max = nodes.iter().map(|x| (*x).borrow().depth).max().unwrap();
    debug!("Min and max depths {y_min}, {y_max}");

    // Keep track of each depth's x position
    let mut x_s = vec![10_f32; y_max - y_min + 1];

    let HEIGHT: u32 = TREE_DEPTH_OFFSET * (y_max - y_min + 2) as u32;
    let fHEIGHT: f32 = HEIGHT as f32;
    let Y_OFFSET: f32 = fHEIGHT / ((y_max - y_min + 2) as f32);
    // Keep track of x,y positions for each node
    let mut plotted_nodes = HashMap::<NodeRef, (f32, f32)>::new();
    // Assign x,y position to each node
    let mut processed_nodes = nodes.iter().map(|node| {
        let depth = (*node).borrow().depth.clone() - y_min;
        let y = ((Y_OFFSET * (depth + 1) as f32 ) as u32) as f32;
        let x = x_s[depth].clone();
        x_s[depth] += X_OFFSET;
        let is_head = (*head) == *node;
        (node.clone(), x.clone(),y.clone(),is_head.clone())
    }).collect::<Vec<(NodeRef, f32, f32, bool)>>();
    // Center each tree depth group
    let last_layer_offset = x_s.last().unwrap().clone()/2_f32;
    let processed_nodes = processed_nodes.iter().map(|(node, x,y,is_head)| {
        let depth = (*node).borrow().depth.clone() - y_min;
        let group_layer_offset = x_s[depth]/2_f32;
        let new_x = *x + last_layer_offset - group_layer_offset;
        plotted_nodes.insert(node.clone(), (new_x.clone(), y.clone()));
        (node.clone(), new_x,*y,*is_head)
    }).collect::<Vec<(NodeRef, f32, f32, bool)>>();

    use plotters::prelude::*;
    
    // Find max float value within f_s
    let fWIDTH: f32 = *x_s.iter().filter(|x| !f32::is_nan(**x)).max_by(|a,b| a.total_cmp(b)).unwrap()+1.5_f32*X_OFFSET;
    let WIDTH: u32 = fWIDTH as u32;

    debug!("Image heigth, image width: {},{}",fHEIGHT, fWIDTH);
    
    let title = String::from(format!("{}.png", title));
    let root = BitMapBackend::new(&title, (WIDTH, HEIGHT)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption("Search Tree", ("sans-serif", 10).into_font())
        // .x_label_area_size(WIDTH)
        // .y_label_area_size(HEIGHT)
        .build_cartesian_2d(0_f32..fWIDTH, fHEIGHT..0_f32)
        .unwrap();

    // Draw nodes
    let _ = chart.draw_series(PointSeries::of_element(
        // vec![(30.0, 60.0), (500.0, 80.0), (700.0, 100.0)],
        processed_nodes.iter().map(|(_, x, y, is_head)| (*x,*y)).collect::<Vec<(f32, f32)>>(),
            NODE_SIZE as i32,
            ShapeStyle::from(&BLACK).filled(),
        &|c, s, st| {
            // let color = if is_head { &RED } else { &BLACK };
            return EmptyElement::at(c)    // We want to construct a composed element on-the-fly
                + Circle::new((0,0),s,st.filled()) // At this point, the new pixel coordinate is established
                + Text::new(format!(""), (0, 0), ("sans-serif", 10).into_font());}
    )).unwrap();

    // Compute edges (child edges and parent edges are the same)
    let mut child_edges: Vec<Vec<(f32,f32)>> = Vec::new();        
    // Gather child edges
    for (node, x, y, is_head) in &processed_nodes{
        for nb in node.borrow().children.iter(){
            let (nbx, nby)= plotted_nodes.get(nb).unwrap();
            let edge = vec![(*x,*y), (*nbx, *nby)];
            child_edges.push(edge);
        }
    }
    //Draw edges
    child_edges.iter().for_each(|x| {chart.draw_series(LineSeries::new((*x).clone(), &RED));});
        
    root.present().unwrap();
}