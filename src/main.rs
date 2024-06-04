use std::error::Error;
use std::sync::Arc;

use indicatif::{ParallelProgressIterator, ProgressStyle};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    ThreadPoolBuilder::new().num_threads(8).build_global().unwrap();

    let allowed_words = Arc::new(get_allowed_words().await?);
    let patterns = Arc::new(patterns());
    let words = allowed_words.clone();

    let mut results: Vec<(String, f32)> = words.par_iter()
        .progress_with_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"))
        .map(|word| {
            let sum = expected_information(word, &allowed_words, &patterns);
            (word.clone(), sum)
        })
        .collect();

    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let expected_information = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("expected_information.txt")
        .await?;
    results.iter().for_each(|(word, e)| writeln!(expected_information, format!("{}={}", word, e)));

    Ok(())
}

async fn get_allowed_words() -> Result<Vec<String>, Box<dyn Error>> {
    let words_list = File::open("allowed_words.txt").await?;
    let reader = BufReader::new(words_list);
    let mut lines = reader.lines();
    let mut allowed_words = Vec::new();
    while let Some(line) = lines.next_line().await? {
        allowed_words.push(line.trim().to_string());
    }
    Ok(allowed_words)
}

fn patterns() -> Vec<Vec<i32>> {
    let grids = 5;
    let colors = vec![1, 2, 3];

    let mut results = Vec::new();
    let mut stack = vec![(0, vec![])];

    while let Some((index, current)) = stack.pop() {
        if index == grids {
            results.push(current);
        } else {
            for &color in &colors {
                let mut next = current.clone();
                next.push(color);
                stack.push((index + 1, next));
            }
        }
    }

    results
}

fn check_word(target_word: &str, word: &str, pattern: &[i32]) -> bool {
    for (i, &pattern) in pattern.iter().enumerate() {
        let letter = target_word.chars().nth(i).unwrap();
        match pattern {
            1 => {
                if word.chars().nth(i).unwrap() != letter {
                    return false;
                }
            }
            2 => {
                if !word.contains(letter) || word.chars().nth(i).unwrap() == letter {
                    return false;
                }
            }
            3 => {
                if word.contains(letter) {
                    return false;
                }
            }
            _ => unreachable!(),
        }
    }
    true
}

fn expected_information(target_word: &str, words: &[String], patterns: &[Vec<i32>]) -> f32 {
    let total_words = words.len() as f32;

    patterns
        .par_iter()
        .map(|pattern| {
            let match_count = words.par_iter().filter(|&word| check_word(target_word, word, pattern)).count() as f32;
            if match_count == 0.0 {
                0.0
            } else {
                let p = match_count / total_words;
                p * p.log2()
            }
        })
        .sum::<f32>()
        * -1.0
}
