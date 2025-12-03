#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#ifdef TERMINAL_MODE
// This code block is compiled only when the TERMINAL_MODE preprocessor directive is defined.
// It provides a simple command-line interface for capturing text.
void run_terminal_mode() {
    char buffer[4096]; // A buffer to hold the input text.
    // Determine the output file path.
    char output_file[256];
    snprintf(output_file, sizeof(output_file), "/tmp/coldvox_terminal_test_%d.txt", getpid());

    // Open the output file for writing.
    FILE *fp = fopen(output_file, "w");
    if (fp == NULL) {
        perror("Failed to open output file");
        exit(1);
    }

    // Read from standard input line by line and write to the file.
    while (fgets(buffer, sizeof(buffer), stdin) != NULL) {
        fprintf(fp, "%s", buffer);
        fflush(fp); // Ensure the text is written immediately.
    }

    fclose(fp);
}
#else
// This code block is compiled when TERMINAL_MODE is not defined.
// It provides a GTK-based graphical interface for capturing text.
#include <gtk/gtk.h>
// This callback is triggered whenever the text in the GtkTextView changes.
static void on_text_changed(GtkTextBuffer *buffer, gpointer user_data) {
    GtkTextIter start, end;
    gtk_text_buffer_get_start_iter(buffer, &start);
    gtk_text_buffer_get_end_iter(buffer, &end);
    gchar *text = gtk_text_buffer_get_text(buffer, &start, &end, FALSE);

    const char *output_file = (const char *)user_data;
    FILE *fp = fopen(output_file, "w");
    if (fp != NULL) {
        fprintf(fp, "%s", text);
        fclose(fp);
    } else {
        perror("Failed to open output file");
    }

    g_free(text);
}

void run_gtk_mode(int argc, char *argv[]) {
    // Determine the output file path.
    char output_file[256];
    snprintf(output_file, sizeof(output_file), "/tmp/coldvox_gtk_test_%d.txt", getpid());

    gtk_init(&argc, &argv);

    GtkWidget *window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "ColdVox Test App");
    gtk_window_set_default_size(GTK_WINDOW(window), 300, 200);
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);

    GtkWidget *textview = gtk_text_view_new();
    GtkTextBuffer *buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(textview));

    GtkWidget *scrolled_window = gtk_scrolled_window_new(NULL, NULL);
    gtk_container_add(GTK_CONTAINER(scrolled_window), textview);
    gtk_container_add(GTK_CONTAINER(window), scrolled_window);

    g_signal_connect(buffer, "changed", G_CALLBACK(on_text_changed), output_file);

    gtk_widget_show_all(window);

    gtk_main();
}
#endif

int main(int argc, char *argv[]) {
#ifdef TERMINAL_MODE
    run_terminal_mode();
#else
    run_gtk_mode(argc, argv);
#endif
    return 0;
}
