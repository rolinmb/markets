import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages
from matplotlib.table import Table
import os

IMGDIR = "img_out"
PDFDIR = "pdf_out"

def add_financials_table(fdf, ax):
    ax.axis('tight')
    ax.axis('off')
    ftable = Table(ax, bbox=[0, 0, 1, 1])
    for (j, label) in enumerate(fdf.columns):
        ftable.add_cell(0, j, width=0.1, height=0.05, text=label, loc='center', facecolor='lightgrey')
    for i, row in enumerate(fdf.itertuples(), start=1):
        for j, value in enumerate(row[1:], start=0):
            ftable.add_cell(i, j, width=0.1, height=0.05, text=value, loc='center')
    ax.add_table(ftable)

if __name__ == "__main__":
    pass
    # TODO: iterate over img_out images and financial data csv to build pdf report of underlying